using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using HarmonyLib;
using MelonLoader;
using MelonLoader.Utils;

[assembly: MelonInfo(typeof(DcgoRecorder.Core), "DCGO Recorder", "0.1.0", "digimon-deck-list-builder")]
[assembly: MelonGame("DCGO", "DCGO")]

namespace DcgoRecorder
{
    /// <summary>
    /// Spike 1 of the MelonLoader recorder (docs/DCGO_MELONLOADER.md). It does two things only:
    ///   1. dumps the real signatures of every game type the Unity-project recorder hooks, to
    ///      UserData/DcgoRecorder/signatures.txt, so the recorder is written against the shipped
    ///      build and not against the (possibly different) submodule source;
    ///   2. applies a PROBE postfix to a few of those methods, to prove Harmony patching works on
    ///      this IL2CPP build, and logs the first calls to UserData/DcgoRecorder/probe.log.
    /// It records nothing yet.
    /// </summary>
    public class Core : MelonMod
    {
        // Types the Unity-project recorder hooks or reads (CLAUDE.md rule 27, docs/DCGO_RECORDING_SCHEMA.md).
        static readonly string[] DumpTypes =
        {
            "TurnStateMachine", "AttackPermanentAction", "PlayCardAction", "UseCardEffect",
            "UserSelectionManager", "SelectCardEffect", "SelectHandEffect", "SelectPermanentEffect",
            "SelectAttackEffect", "SelectCountEffect", "SelectDigiXrosClass", "MultipleSkills",
            "OptionalSkill", "PlayCardClass", "DrawClass", "ICardEffect", "ActivateClass",
            "AutoProcessing", "Permanent", "CardSource", "Player", "GManager",
            "Opening", "HomeMode", "DeckMode", "BattleMode", "Title", "RoomManager", "CheckUpdate",
            "_PlayCard_d__45", "_BreedingPhase_d__28", "AttackProcess", "GameContext", "EnterRoom", "SelectBattleMode", "OpeningButton", "YesNoObject", "SelectBattleDeck", "SelectRandomDeckButton", "CommandButton", "DeckSelectButton", "DeckData", "ContinuousController", "BattleScene",
        };

        // (type, method) pairs whose calls are written to the raw event log. Every selection result
        // and every main-phase action reaches the game through one of these plain methods (the same
        // calls the game uses to sync a match between two clients), so no coroutine has to be wrapped.
        static readonly (string Type, string Method)[] Probes =
        {
            // main-phase actions: local UI, the opponent's serialized packet, the AI/script mirrors
            ("TurnStateMachine", "QueueMainPhaseAction"),
            ("TurnStateMachine", "QueueMainPhaseAction_Internal"),
            ("TurnStateMachine", "SetActSkill"),
            ("TurnStateMachine", "SetActCardSkill"),
            ("TurnStateMachine", "SetAttackingPermaent"),
            ("TurnStateMachine", "SetBreedingPhase"),
            // game lifecycle
            ("TurnStateMachine", "StartGame"),
            ("TurnStateMachine", "EndGame"),
            ("TurnStateMachine", "Surrender"),
            ("TurnStateMachine", "PassTurn"),
            ("TurnStateMachine", "SetRedraw"),
            // the bot (and any scripted seat) never calls QueueMainPhaseAction: it writes these fields directly
            ("TurnStateMachine", "set_PlayCard"),
            ("TurnStateMachine", "set_AttackingPermanent"),
            ("TurnStateMachine", "set_UseCardEffect"),
            ("TurnStateMachine", "set_Passed"),
            ("TurnStateMachine", "Init"),
            ("Permanent", "AddCardSource"),
            ("TurnStateMachine", "SetStartPlayer"),
            // selection results (both seats)
            ("SelectCardEffect", "SetTargetCardAndIndicies"),
            ("SelectHandEffect", "SetTargetHandCards"),
            ("SelectPermanentEffect", "SetTargetFrames"),
            ("SelectAttackEffect", "SetAttackTarget"),
            ("SelectDigiXrosClass", "SetTargetDigiXrossIndex"),
            // the generic int / bool selection channel
            ("UserSelectionManager", "SetIntForPlayer"),
            ("MultipleSkills", "SetTargetSkill"),
            ("UserSelectionManager", "SetBoolForPlayer"),
            // "use this optional effect?" (Use / back button), an RPC on both clients
            ("OptionalSkill", "SetUseOptional"),
        };

        static string _dir;
        static readonly object Gate = new object();
        static readonly Dictionary<string, int> Seen = new Dictionary<string, int>();

        // ---- scene / button dump (read-only): what is on screen, so a match can later be started
        // from inside the process instead of by hand. -----------------------------------------
        string _pendingSceneDump;
        float _dumpAt;

        public override void OnSceneWasLoaded(int buildIndex, string sceneName)
        {
            _pendingSceneDump = sceneName;
            _dumpAt = UnityEngine.Time.realtimeSinceStartup + 4f; // let the scene's UI finish building
            LoggerInstance.Msg("scene loaded: " + sceneName);
            if (sceneName == "BattleScene")
            {
                _planCall = 0;
                _remoteStampIdx[0] = _remoteStampIdx[1] = 0;
                // a fresh game: this client's own seat stamps start empty (each client only ever writes its own seat)
                try
                {
                    var gm = Il2Cpp.GManager.instance;
                    if (_stampDir != null && _autoMode == "host") File.Delete(Path.Combine(_stampDir, "stamps-0.txt"));
                    if (_stampDir != null && _autoMode == "join") File.Delete(Path.Combine(_stampDir, "stamps-1.txt"));
                    if (_stampDir != null && _autoMode == "host") File.Delete(Path.Combine(_stampDir, "chosen-0.txt"));
                    if (_stampDir != null && _autoMode == "join") File.Delete(Path.Combine(_stampDir, "chosen-1.txt"));
                    _remoteChosenIdx[0] = _remoteChosenIdx[1] = 0;
                }
                catch { }
            }
            if (sceneName == "ContinuousControllerScene")
            {
                _loaderSeen = true;
                _autoAt = UnityEngine.Time.realtimeSinceStartup + 12f;
            }
        }

        // ---- scripted menu walk (sandbox only): one step per call, results read from the logs ----------
        bool _loaderSeen;
        string _autoMode;   // null = not read yet, "" = off, else bot | host | join
        string _sharedDir;  // where a room host publishes the room id for the joining client
        int _autoStage;
        int _autoAttempts;
        float _autoAt = -1;

        void AutoStep(float now)
        {
            // Only start once the game's loader scene has finished and had time to settle: acting earlier
            // races its initialisation, which resets the menu.
            if (!_loaderSeen) return;
            if (now < _autoAt) return;
            if (_autoMode == null)
            {
                // The scripted walk is OPT-IN: without UserData/DcgoRecorder/auto.txt the mod only records, so a
                // normal play session (or the real install) is never driven by it.
                var cfg = Path.Combine(_dir, "auto.txt");
                if (!File.Exists(cfg)) { _autoMode = ""; LoggerInstance.Msg("auto: no auto.txt - recording only"); }
                else
                {
                    var lines = File.ReadAllLines(cfg);
                    _autoMode = lines.Length > 0 ? lines[0].Trim().ToLowerInvariant() : "bot";
                    _sharedDir = lines.Length > 1 ? lines[1].Trim() : _dir;
                    LoggerInstance.Msg("auto: mode " + _autoMode + ", shared dir " + _sharedDir);
                    _stampDir = _sharedDir;
                }
            }
            if (_autoMode == "") return;
            try
            {
                switch (_autoStage)
                {
                    case 0:
                        {
                            var title = UnityEngine.Object.FindObjectOfType<Il2Cpp.Title>();
                            LoggerInstance.Msg("auto: Title found = " + (title != null));
                            if (title != null) title.OnClick();
                            _autoStage = 1; _autoAt = now + 10f; _autoAttempts = 0;
                            break;
                        }
                    case 1:
                        {
                            LoggerInstance.Msg("auto: re-dumping tree after title click");
                            _treeDumped.Remove("Opening/after-title-click");
                            DumpTreeNow("Opening/after-title-click");
                            _autoStage = 2; _autoAt = now + 2f;
                            break;
                        }
                    case 2:
                        {
                            ClickEventTrigger("Opening/Main Menu/ModeButtons/BattleButton/Button");
                            _autoStage = 3; _autoAt = now + 8f;
                            break;
                        }
                    case 3:
                        {
                            LoggerInstance.Msg("auto: re-dumping tree after battle click");
                            DumpTreeNow("Opening/after-battle-click");
                            _autoStage = 4; _autoAt = now + 2f; _autoAttempts = 0;
                            break;
                        }
                    case 4:
                        {
                            var sbm = UnityEngine.Object.FindObjectOfType<Il2Cpp.SelectBattleMode>();
                            if (sbm == null) throw new Exception("SelectBattleMode not found yet");
                            if (_autoMode == "host")
                            {
                                LoggerInstance.Msg("auto: SelectBattleMode.StartCreateRoom()");
                                sbm.StartCreateRoom();
                                _autoStage = 30; _autoAt = now + 6f; _autoAttempts = 0;
                                break;
                            }
                            if (_autoMode == "join")
                            {
                                _autoStage = 40; _autoAt = now + 1f; _autoAttempts = 0;
                                break;
                            }
                            LoggerInstance.Msg("auto: SelectBattleMode.StartSelectBattleDeck(isAI: true)");
                            sbm.StartSelectBattleDeck(true);
                            _autoStage = 5; _autoAt = now + 8f; _autoAttempts = 0;
                            break;
                        }
                    case 5:
                        {
                            LoggerInstance.Msg("auto: re-dumping tree after bot-match click");
                            DumpTreeNow("Opening/after-botmatch-click");
                            _autoStage = 6; _autoAt = now + 2f; _autoAttempts = 0;
                            break;
                        }
                    case 6:
                        {
                            // Pick the first saved deck (DeckInfoPrefab clone) in the deck list.
                            var dk = UnityEngine.Object.FindObjectOfType<Il2Cpp.DeckInfoPrefab>();
                            if (dk == null) throw new Exception("DeckInfoPrefab not found yet");
                            var bg = dk.transform.Find("BackGround");
                            if (bg == null) throw new Exception("DeckInfoPrefab/BackGround missing");
                            var trigger = bg.GetComponent<UnityEngine.EventSystems.EventTrigger>();
                            LoggerInstance.Msg("auto: selecting deck " + PathOf(dk.transform) + " (EventTrigger=" + (trigger != null) + ")");
                            if (trigger != null)
                                trigger.OnPointerClick(new UnityEngine.EventSystems.PointerEventData(UnityEngine.EventSystems.EventSystem.current));
                            _autoStage = 7; _autoAt = now + 3f; _autoAttempts = 0;
                            break;
                        }
                    case 7:
                        {
                            ClickUiButtonNamed("SelectButton");
                            _autoStage = 8; _autoAt = now + 8f; _autoAttempts = 0;
                            break;
                        }
                    case 8:
                        {
                            // Wait for an actual match: GManager has a turn state machine and the local player.
                            var gm = Il2Cpp.GManager.instance;
                            var tsm = gm != null ? gm.turnStateMachine : null;
                            if (gm == null || tsm == null || gm.You == null)
                            {
                                if (_autoAttempts == 0) DumpTreeNow("Opening/after-select-click");
                                if (_autoAttempts == 3)
                                {
                                    var cc = Il2Cpp.ContinuousController.instance;
                                    LoggerInstance.Msg("auto: ContinuousController.BattleDeckData null=" + (cc.BattleDeckData == null) + " DeckDatas=" + (cc.DeckDatas == null ? -1 : cc.DeckDatas.Count));
                                    if (cc.BattleDeckData == null && cc.DeckDatas != null && cc.DeckDatas.Count > 0)
                                    {
                                        cc.BattleDeckData = cc.DeckDatas[0];
                                        LoggerInstance.Msg("auto: set BattleDeckData = DeckDatas[0]");
                                    }
                                    var sbd = UnityEngine.Object.FindObjectOfType<Il2Cpp.SelectBattleDeck>();
                                    LoggerInstance.Msg("auto: no match yet; calling SelectBattleDeck.OnClickSelectButton_BotMatch directly (found=" + (sbd != null) + ")");
                                    if (sbd != null) sbd.OnClickSelectButton_BotMatch();
                                }
                                if (_autoAttempts++ % 6 == 0) LoggerInstance.Msg("auto: waiting for a match to start (scene " + _currentScene + ")");
                                if (_autoAttempts > 60) { LoggerInstance.Warning("auto: no match started"); _autoStage = 99; }
                                _autoAt = now + 5f;
                                break;
                            }
                            LoggerInstance.Msg("auto: match is running; IsAI=" + gm.IsAI + " you=" + gm.You.PlayerID + " turn=" + tsm.TurnCount);
                            _autoStage = _agentDir != null && _agentDir != "" ? 9 : 11; _autoAt = now + 8f; _autoAttempts = 0;
                            break;
                        }
                    case 11:
                        {
                            if (System.IO.File.Exists(System.IO.Path.Combine(MelonLoader.Utils.MelonEnvironment.UserDataDirectory, "DcgoRecorder", "play.flag")))
                            {
                                LoggerInstance.Msg("auto: play.flag present - leaving the match to the human player");
                                _autoStage = 99;
                                break;
                            }
                            // The game parks on the "First Player" choice before anything else happens.
                            ClickUiButtonNamed("LargeButton02Yellow");
                            _autoStage = 12; _autoAt = now + 12f; _autoAttempts = 0;
                            break;
                        }
                    case 12:
                        {
                            // Then the mulligan prompt: keep the opening hand.
                            ClickUiButtonNamed("NotSelectButton");
                            _autoStage = 9; _autoAt = now + 60f; _autoAttempts = 0;
                            break;
                        }
                    case 9:
                        {
                            var gm = Il2Cpp.GManager.instance;
                            var tsm = gm.turnStateMachine;
                            // A human wants to play this game: leave a file UserData/DcgoRecorder/play.flag to skip the surrender.
                            if (System.IO.File.Exists(System.IO.Path.Combine(MelonLoader.Utils.MelonEnvironment.UserDataDirectory, "DcgoRecorder", "play.flag")))
                            {
                                LoggerInstance.Msg("auto: play.flag present - leaving the match to the human player");
                                _autoStage = 99;
                                break;
                            }
                            if (_agentDir != null && _agentDir != "")
                            {
                                if (_gameEnded || _autoAttempts > 1200) { _autoStage = 10; _autoAt = now + 5f; break; }
                                if (_autoAttempts++ % 24 == 0) LoggerInstance.Msg("auto: model seat playing (turn " + tsm.TurnCount + ")");
                                _autoAt = now + 5f;
                                break;
                            }
                            // Play on by pressing "Next Phase" (Btn_Skill00) until a few turns have passed; that walks the
                            // breeding / main / end phases against the bot with no card plays - enough to exercise the hooks.
                            if (tsm.IsSelecting)
                                LoggerInstance.Msg("auto: a selection prompt is open (turn " + tsm.TurnCount + "); the script cannot answer it, ending the game");
                            else if (tsm.TurnCount < 6 && _autoAttempts < 40)
                            {
                                _autoAttempts++;
                                // only on the local seat's own turn: a click during the bot's turn races the bot's own flow
                                if (TurnPlayerId() == gm.You.PlayerID)
                                {
                                    try { ClickUiButtonNamed("Btn_Skill00"); }
                                    catch (Exception) { /* not shown right now (a prompt is up): try again */ }
                                }
                                _autoAt = now + 5f;
                                break;
                            }
                            if (_autoMode == "join")
                            {
                                if (_autoAttempts < 200) { _autoAttempts++; _autoAt = now + 5f; break; } // the host surrenders
                                _autoStage = 10; _autoAt = now + 1f;
                                break;
                            }
                            LoggerInstance.Msg("auto: surrendering as player " + gm.You.PlayerID + " (turn " + tsm.TurnCount + ", ticks " + _autoAttempts + ")");
                            tsm.Surrender(gm.You.PlayerID);
                            _autoStage = 10; _autoAt = now + 15f;
                            break;
                        }
                    case 30:
                        {
                            // host: wait for the room id, publish it for the joining client
                            var rm = UnityEngine.Object.FindObjectOfType<Il2Cpp.RoomManager>();
                            var shown = rm != null && rm.RoomIDText != null ? rm.RoomIDText.text : null;
                            var (inRoom, roomName, players) = PhotonRoom();
                            LoggerInstance.Msg("auto: photon inRoom=" + inRoom + " room='" + roomName + "' players=" + players + " shown='" + shown + "'");
                            if (!inRoom || string.IsNullOrWhiteSpace(roomName)) throw new Exception("not in a Photon room yet");
                            var id = roomName;
                            Directory.CreateDirectory(_sharedDir);
                            File.WriteAllText(Path.Combine(_sharedDir, "room.txt"), id.Trim());
                            LoggerInstance.Msg("auto: room created, id '" + id.Trim() + "' -> " + Path.Combine(_sharedDir, "room.txt"));
                            DumpTreeNow("Opening/room-host");
                            _autoStage = 50; _autoAt = now + 3f; _autoAttempts = 0;
                            break;
                        }
                    case 40:
                        {
                            // join: wait for the host's room id, then open the enter-room window
                            var f = Path.Combine(_sharedDir, "room.txt");
                            if (!File.Exists(f)) throw new Exception("no room id published yet at " + f);
                            var id = File.ReadAllText(f).Trim();
                            var sbm = UnityEngine.Object.FindObjectOfType<Il2Cpp.SelectBattleMode>();
                            if (sbm == null) throw new Exception("SelectBattleMode not found yet");
                            LoggerInstance.Msg("auto: joining room '" + id + "': SelectBattleMode.StartEnterRoomID()");
                            sbm.StartEnterRoomID();
                            _autoStage = 41; _autoAt = now + 4f; _autoAttempts = 0;
                            break;
                        }
                    case 41:
                        {
                            var er = UnityEngine.Object.FindObjectOfType<Il2Cpp.EnterRoom>();
                            if (er == null || er.RoomIDInputField == null) throw new Exception("EnterRoom window not up yet");
                            var id = File.ReadAllText(Path.Combine(_sharedDir, "room.txt")).Trim();
                            er.RoomIDInputField.text = id;
                            LoggerInstance.Msg("auto: EnterRoom '" + id + "' -> OnClickEnterRoomButton()");
                            er.OnClickEnterRoomButton();
                            _autoStage = 42; _autoAt = now + 4f; _autoAttempts = 0;
                            break;
                        }
                    case 42:
                        {
                            var (inRoom, roomName, players) = PhotonRoom();
                            LoggerInstance.Msg("auto: photon inRoom=" + inRoom + " room='" + roomName + "' players=" + players);
                            if (!inRoom || players < 2) throw new Exception("not in the host's room yet");
                            _autoStage = 50; _autoAt = now + 3f; _autoAttempts = 0;
                            break;
                        }
                    case 50:
                        {
                            // both seats, in the room: open the battle-deck picker
                            var rm = UnityEngine.Object.FindObjectOfType<Il2Cpp.RoomManager>();
                            if (rm == null) throw new Exception("RoomManager not found");
                            if (_plan != null && _autoMode == "host")
                            {
                                LoggerInstance.Msg("auto: room: playback first player index " + _planFirst + " (0 = master)");
                                rm.OnClickFirstPlayerIndexIDButton(_planFirst);
                            }
                            LoggerInstance.Msg("auto: room: OnClickSelectBattleDeckButton()");
                            rm.OnClickSelectBattleDeckButton();
                            _autoStage = 51; _autoAt = now + 4f; _autoAttempts = 0;
                            break;
                        }
                    case 51:
                        {
                            var dk = UnityEngine.Object.FindObjectOfType<Il2Cpp.DeckInfoPrefab>();
                            if (dk == null) throw new Exception("room deck list not shown yet");
                            var bg = dk.transform.Find("BackGround");
                            var trigger = bg != null ? bg.GetComponent<UnityEngine.EventSystems.EventTrigger>() : null;
                            LoggerInstance.Msg("auto: room: selecting deck " + PathOf(dk.transform));
                            if (trigger != null)
                                trigger.OnPointerClick(new UnityEngine.EventSystems.PointerEventData(UnityEngine.EventSystems.EventSystem.current));
                            _autoStage = 52; _autoAt = now + 3f; _autoAttempts = 0;
                            break;
                        }
                    case 52:
                        {
                            ClickUiButtonNamed("SelectButton");
                            _autoStage = 53; _autoAt = now + 5f; _autoAttempts = 0;
                            break;
                        }
                    case 53:
                        {
                            var rm = UnityEngine.Object.FindObjectOfType<Il2Cpp.RoomManager>();
                            if (rm == null) throw new Exception("RoomManager not found");
                            if (!rm.CanReady()) throw new Exception("cannot ready yet (no deck signed up / opponent missing)");
                            LoggerInstance.Msg("auto: room: OnClickGetReadyButton()");
                            rm.OnClickGetReadyButton();
                            DumpTreeNow("Opening/room-ready-" + _autoMode);
                            _autoStage = 54; _autoAt = now + 5f; _autoAttempts = 0;
                            break;
                        }
                    case 54:
                        {
                            // wait for the battle; the host may have to start it once both seats are ready
                            var gm = Il2Cpp.GManager.instance;
                            if (gm != null && gm.turnStateMachine != null && gm.You != null)
                            {
                                LoggerInstance.Msg("auto: PvP match is running; IsAI=" + gm.IsAI + " you=" + gm.You.PlayerID);
                                _autoStage = _agentDir != null && _agentDir != "" ? 9 : 11; _autoAt = now + 8f; _autoAttempts = 0;
                                break;
                            }
                            var rm = UnityEngine.Object.FindObjectOfType<Il2Cpp.RoomManager>();
                            var (inRoom2, roomName2, players2) = PhotonRoom();
                            if (_autoAttempts % 6 == 0) LoggerInstance.Msg("auto: photon inRoom=" + inRoom2 + " room='" + roomName2 + "' players=" + players2);
                            if (_autoMode == "host" && players2 >= 2 && rm != null && rm.AllPlayerIsReady() && !rm.DoneStartBattle && _autoAttempts >= 6 && _autoAttempts % 6 == 0)
                            {
                                LoggerInstance.Msg("auto: room: both ready, host calls GoToBattleScene()");
                                rm.GoToBattleScene();
                            }
                            if (_autoAttempts++ % 6 == 0) LoggerInstance.Msg("auto: waiting for the PvP battle (allReady=" + (rm != null && rm.AllPlayerIsReady()) + ")");
                            if (_autoAttempts > 72) { LoggerInstance.Warning("auto: PvP battle never started"); _autoStage = 99; }
                            _autoAt = now + 5f;
                            break;
                        }
                    case 10:
                        {
                            LoggerInstance.Msg("auto: done (" + _currentScene + ")");
                            _autoStage = 99;
                            break;
                        }
                }
            }
            catch (Exception e)
            {
                // The game may still be initialising (a null reference inside its own handler): retry.
                _autoAttempts++;
                var first = e.Message.Split('\n')[0];
                LoggerInstance.Warning("auto step " + _autoStage + " attempt " + _autoAttempts + " failed: " + first);
                if ((_autoStage == 11 || _autoStage == 12) && _autoAttempts >= 6)
                {
                    LoggerInstance.Msg("auto: no " + (_autoStage == 11 ? "First Player" : "Keep Hand") + " prompt, moving on");
                    _autoStage = _autoStage == 11 ? 12 : 9; _autoAttempts = 0; _autoAt = now + 3f;
                }
                else if (_autoAttempts >= 12) { LoggerInstance.Warning("auto: giving up"); _autoStage = 99; }
                else _autoAt = now + 5f;
            }
        }

        // Invoke a standard UnityEngine.UI.Button's onClick, found by object name (menu paths contain '/').
        void ClickUiButtonNamed(string name)
        {
            foreach (var b in UnityEngine.Object.FindObjectsOfType<UnityEngine.UI.Button>())
            {
                if (b == null || b.name != name || !b.gameObject.activeInHierarchy) continue;
                LoggerInstance.Msg("auto: invoking " + PathOf(b.transform) + " interactable=" + b.interactable);
                b.onClick.Invoke();
                return;
            }
            throw new Exception("button '" + name + "' not found yet");
        }

        // Fire the pointer-click of an EventTrigger the way the EventSystem would (no screen input).
        void ClickEventTrigger(string path)
        {
            var go = UnityEngine.GameObject.Find(path);
            LoggerInstance.Msg("auto: click " + path + " -> " + (go != null ? "found" : "NOT FOUND"));
            if (go == null) return;
            var trigger = go.GetComponent<UnityEngine.EventSystems.EventTrigger>();
            if (trigger == null) { LoggerInstance.Warning("auto: no EventTrigger on " + path); return; }
            var data = new UnityEngine.EventSystems.PointerEventData(UnityEngine.EventSystems.EventSystem.current);
            trigger.OnPointerClick(data);
        }

        string _currentScene = "?";
        string _lastDumpSignature = "";
        float _nextPeriodic;

        bool _postMulliganDone;

        public override void OnUpdate()
        {
            var now = UnityEngine.Time.realtimeSinceStartup;
            try
            {
                // how long DCGO's card panel has been READY: it is shown before its setup coroutine ends (the card
                // list "Parent" is switched on last). Closing it earlier (End Selection) deactivates the panel and kills
                // that coroutine, and DCGO then waits forever on a selection with no UI.
                var pnl = Il2Cpp.GManager.instance?.selectCardPanel;
                bool open = pnl != null && pnl.gameObject.activeInHierarchy && pnl.Parent != null && pnl.Parent.activeInHierarchy;
                if (!open) _panelOpenSince = float.MaxValue;
                else if (_panelOpenSince == float.MaxValue) _panelOpenSince = now;
            }
            catch { }
            AutoStep(now);
            AgentStep(now);
            // The post-mulligan zones (what the schema's `initial_state` row carries): written once per game, the
            // first frame DoneStartGame is true - after the redraws and the security deal, before turn 1 moves on.
            try
            {
                var gmx = Il2Cpp.GManager.instance;
                var tsmx = gmx != null ? gmx.turnStateMachine : null;
                if (tsmx != null && tsmx.DoneStartGame)
                {
                    if (!_postMulliganDone) { _postMulliganDone = true; WriteSnapshot("PostMulligan"); }
                }
                else _postMulliganDone = false; // a new game (DoneStartGame resets) arms it again
            }
            catch { }
            if (_pendingSceneDump != null && now >= _dumpAt)
            {
                _currentScene = _pendingSceneDump;
                _pendingSceneDump = null;
                _nextPeriodic = now; // dump straight away
            }
            // Re-dump every 15 s, but only write when the set of buttons changed (menus build over time).
            if (_currentScene != "?" && now >= _nextPeriodic)
            {
                _nextPeriodic = now + 15f;
                try { DumpButtons(_currentScene); } catch (Exception e) { LoggerInstance.Warning("button dump failed: " + e.Message); }
            }
        }

        static string PathOf(UnityEngine.Transform t)
        {
            var parts = new List<string>();
            for (var c = t; c != null; c = c.parent) parts.Add(c.name);
            parts.Reverse();
            return string.Join("/", parts);
        }

        // Object tree with component type names, once per scene: DCGO's menus are not UnityEngine.UI
        // buttons, so this shows which components handle a click.
        readonly HashSet<string> _treeDumped = new HashSet<string>();

        void DumpTree(string scene)
        {
            if (!_treeDumped.Add(scene)) return;
            DumpTreeNow(scene);
        }

        void DumpTreeNow(string scene)
        {
            var sb = new StringBuilder();
            sb.AppendLine("# tree of scene " + scene + " at " + DateTime.UtcNow.ToString("o"));
            int lines = 0;
            void Walk(UnityEngine.Transform t, int depth)
            {
                if (t == null || lines > 2500 || depth > 11) return;
                if (!t.gameObject.activeInHierarchy) return;
                var comps = new List<string>();
                try
                {
                    foreach (var c in t.gameObject.GetComponents<UnityEngine.Component>())
                    {
                        if (c == null) continue;
                        var n = c.ToString();
                        if (n.Contains("(UnityEngine.Transform)") || n.Contains("(UnityEngine.RectTransform)")
                            || n.Contains("CanvasRenderer") || n.Contains("(UnityEngine.UI.Image)")
                            || n.Contains("(UnityEngine.UI.Text)")) continue;
                        var open = n.LastIndexOf('(');
                        comps.Add(open >= 0 ? n.Substring(open).Trim('(', ')') : n);
                    }
                }
                catch { }
                string label = "";
                try
                {
                    var tmp = t.gameObject.GetComponent<Il2CppTMPro.TMP_Text>();
                    if (tmp != null && !string.IsNullOrWhiteSpace(tmp.text)) label = "  \"" + tmp.text.Replace("\n", " ") + "\"";
                }
                catch { }
                sb.AppendLine(new string(' ', depth * 2) + t.name + (comps.Count > 0 ? "  [" + string.Join(", ", comps) + "]" : "") + label);
                lines++;
                for (int i = 0; i < t.childCount; i++) Walk(t.GetChild(i), depth + 1);
            }
            for (int si = 0; si < UnityEngine.SceneManagement.SceneManager.sceneCount; si++)
            {
                var sc = UnityEngine.SceneManagement.SceneManager.GetSceneAt(si);
                sb.AppendLine("=== loaded scene [" + si + "] " + sc.name + " (roots " + sc.rootCount + ")");
                foreach (var go in sc.GetRootGameObjects()) Walk(go.transform, 0);
            }
            // Objects marked DontDestroyOnLoad live in a hidden scene; reach them through any such root.
            try
            {
                var dd = UnityEngine.Object.FindObjectsOfType<UnityEngine.Canvas>()
                    .Where(c => c != null && c.transform.parent == null && c.gameObject.scene.name == "DontDestroyOnLoad").ToList();
                sb.AppendLine("=== DontDestroyOnLoad canvases: " + dd.Count);
                foreach (var c in dd) Walk(c.transform, 0);
            }
            catch (Exception e) { sb.AppendLine("(DDOL scan failed: " + e.Message + ")"); }
            File.AppendAllText(Path.Combine(_dir, "tree.txt"), sb.ToString() + "\n");
            LoggerInstance.Msg("tree of " + scene + ": " + lines + " objects");
        }

        void DumpButtons(string scene)
        {
            try { DumpTree(scene); } catch (Exception e) { LoggerInstance.Warning("tree dump failed: " + e.Message); }
            var sb = new StringBuilder();
            sb.AppendLine("# scene " + scene + " at " + DateTime.UtcNow.ToString("o"));
            var buttons = UnityEngine.Object.FindObjectsOfType<UnityEngine.UI.Button>();
            sb.AppendLine("buttons: " + buttons.Length);
            foreach (var b in buttons)
            {
                if (b == null) continue;
                string label = "";
                try
                {
                    var txt = b.GetComponentInChildren<UnityEngine.UI.Text>();
                    if (txt != null) label = txt.text;
                }
                catch { }
                sb.AppendLine("  " + (b.gameObject.activeInHierarchy ? "[on] " : "[off] ") + PathOf(b.transform)
                    + (b.interactable ? "" : " (not interactable)") + (label.Length > 0 ? "  \"" + label.Replace("\n", " ") + "\"" : ""));
            }
            // Everything visible: UI Text strings and the root objects of the scene, so a screen the
            // button scan cannot see (a login or update dialog built from other widgets) still shows up.
            try
            {
                var texts = UnityEngine.Object.FindObjectsOfType<UnityEngine.UI.Text>();
                sb.AppendLine("texts: " + texts.Length);
                foreach (var t in texts.Where(t => t != null && t.gameObject.activeInHierarchy && !string.IsNullOrWhiteSpace(t.text)).Take(40))
                    sb.AppendLine("  text " + PathOf(t.transform) + "  \"" + t.text.Replace("\n", " ") + "\"");
            }
            catch (Exception e) { sb.AppendLine("  (text scan failed: " + e.Message + ")"); }
            try
            {
                var selectables = UnityEngine.Object.FindObjectsOfType<UnityEngine.UI.Selectable>();
                sb.AppendLine("selectables: " + selectables.Length);
                foreach (var t in selectables.Where(t => t != null && t.gameObject.activeInHierarchy).Take(40))
                    sb.AppendLine("  selectable " + t.GetType().Name + " " + PathOf(t.transform));
                var canvases = UnityEngine.Object.FindObjectsOfType<UnityEngine.Canvas>();
                sb.AppendLine("canvases: " + canvases.Length);
                foreach (var c in canvases.Where(c => c != null && c.gameObject.activeInHierarchy).Take(20))
                    sb.AppendLine("  canvas " + PathOf(c.transform));
            }
            catch (Exception e) { sb.AppendLine("  (selectable scan failed: " + e.Message + ")"); }
            var body = sb.ToString();
            var signature = scene + "|" + string.Join("\n", body.Split('\n').Skip(1));
            if (signature == _lastDumpSignature) return;
            _lastDumpSignature = signature;
            File.AppendAllText(Path.Combine(_dir, "scenes.txt"), body + "\n");
            LoggerInstance.Msg("dumped " + buttons.Length + " buttons for scene " + scene);
        }

        public override void OnInitializeMelon()
        {
            _dir = Path.Combine(MelonEnvironment.UserDataDirectory, "DcgoRecorder");
            Directory.CreateDirectory(_dir);
            try { DumpSignatures(); } catch (Exception e) { LoggerInstance.Error("signature dump failed: " + e); }
            try { ApplyProbes(); } catch (Exception e) { LoggerInstance.Error("probe patching failed: " + e); }
            try { LoadPlaybackPlan(); } catch (Exception e) { LoggerInstance.Error("playback plan failed: " + e); }
        }

        // ---- playback: play a game our engine played (docs/DCGO_MELONLOADER.md "Playback") ------------------------
        // Opt-in by UserData/DcgoRecorder/playback.txt (one line: a plan written by make_playback_plan.py). The plan is
        // line-based: first player (0 = the room's master, 1 = the joiner), then the main deck and egg deck of seat 0
        // and seat 1, each a comma-separated top-first card-id list. Every client in the match must load the SAME plan:
        // each one simulates both decks, so the forced orders keep them in step exactly as DCGO's shared seed did.
        static int _engineAction = -1;  // see AnswerSelection
        static string _stampDir;          // shared dir of the two PvP clients (auto.txt line 2)
        static readonly int[] _remoteStampIdx = new int[2];

        static readonly int[] _remoteChosenIdx = new int[2];

        // the trigger-order pick (MultipleSkills.SetTargetSkill) names its effect by an index into a list only the
        // picking client shows; that client writes the picked effect's card here, in order, for the other client
        static string ChosenFile(int pid) => _stampDir == null ? null : Path.Combine(_stampDir, "chosen-" + pid + ".txt");

        static void AppendChosen(int pid, string card)
        {
            try { var f = ChosenFile(pid); if (f != null) File.AppendAllText(f, (card ?? "-") + Environment.NewLine); } catch { }
        }

        static string ReadChosen(int pid, int index)
        {
            try
            {
                var f = ChosenFile(pid);
                if (f == null) return null;
                for (int tries = 0; tries < 20; tries++)
                {
                    if (File.Exists(f))
                    {
                        var lines = File.ReadAllLines(f);
                        if (index < lines.Length) { var v = lines[index].Trim(); return v == "-" ? null : v; }
                    }
                    System.Threading.Thread.Sleep(25);
                }
            }
            catch { }
            return null;
        }

        static string StampFile(int pid) => _stampDir == null ? null : Path.Combine(_stampDir, "stamps-" + pid + ".txt");

        static void AppendStamp(int pid, int ea)
        {
            try { var f = StampFile(pid); if (f != null) File.AppendAllText(f, ea + Environment.NewLine); } catch { }
        }

        static int ReadStamp(int pid, int index)
        {
            try
            {
                var f = StampFile(pid);
                if (f == null) return -1;
                // the answering client writes before its RPC reaches us; give a slow disk a moment
                for (int tries = 0; tries < 20; tries++)
                {
                    if (File.Exists(f))
                    {
                        var lines = File.ReadAllLines(f);
                        if (index < lines.Length) return int.TryParse(lines[index].Trim(), out var v) ? v : -1;
                    }
                    System.Threading.Thread.Sleep(25);
                }
            }
            catch { }
            return -1;
        }
        static string[][] _plan;      // main0, egg0, main1, egg1
        static int _planFirst = -1;
        static int _planCall;         // n-th deck build of this game (CardObjectController: master main, master egg, joiner main, joiner egg)
        static Dictionary<string, Il2Cpp.CEntity_Base> _entities;

        void LoadPlaybackPlan()
        {
            var cfg = Path.Combine(_dir, "playback.txt");
            if (!File.Exists(cfg)) return;
            var path = File.ReadAllText(cfg).Trim();
            var lines = File.ReadAllLines(path);
            if (lines.Length < 5) throw new Exception("plan " + path + " needs 5 lines (first player, main0, egg0, main1, egg1)");
            _planFirst = int.Parse(lines[0].Trim());
            _plan = new string[4][];
            for (int i = 0; i < 4; i++)
                _plan[i] = lines[1 + i].Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries).Select(x => x.Trim()).ToArray();
            var asm = GameAssembly();
            int patched = 0;
            foreach (var t in FindTypes(asm, "RandomUtility"))
                foreach (var m in t.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly).Where(x => x.Name == "ShuffledDeckCards"))
                {
                    var arg = m.GetParameters().FirstOrDefault()?.ParameterType.ToString() ?? "";
                    var post = arg.Contains("CEntity_Base") ? nameof(ShuffleEntitiesPostfix) : arg.Contains("CardSource") ? nameof(ShuffleSourcesPostfix) : null;
                    if (post == null) continue;
                    HarmonyInstance.Patch(m, postfix: new HarmonyMethod(typeof(Core).GetMethod(post, BindingFlags.Static | BindingFlags.NonPublic)));
                    patched++;
                }
            LoggerInstance.Msg("playback: plan " + path + " (first player " + _planFirst + ", decks " + string.Join("/", _plan.Select(x => x.Length)) + "), shuffles patched: " + patched);
        }

        static Il2Cpp.CEntity_Base Entity(string id)
        {
            if (_entities == null)
            {
                _entities = new Dictionary<string, Il2Cpp.CEntity_Base>();
                foreach (var e in Il2Cpp.ContinuousController.instance.CardList)
                    if (e != null && !string.IsNullOrEmpty(e.CardID) && !_entities.ContainsKey(e.CardID)) _entities[e.CardID] = e;
            }
            return _entities.TryGetValue(id, out var x) ? x : null;
        }

        // A deck being built: replace it with the plan's cards, in the plan's order (the deck chosen in the room only
        // decides that a deck exists).
        static void ShuffleEntitiesPostfix(ref Il2CppSystem.Collections.Generic.List<Il2Cpp.CEntity_Base> __result)
        {
            if (_plan == null || _planCall >= 4) return;
            int which = _planCall++;
            var list = new Il2CppSystem.Collections.Generic.List<Il2Cpp.CEntity_Base>();
            foreach (var id in _plan[which])
            {
                var e = Entity(id);
                if (e == null) { MelonLogger.Error("playback: DCGO has no card " + id + " - deck " + which + " left as shuffled"); return; }
                list.Add(e);
            }
            MelonLogger.Msg("playback: deck " + which + " (" + (which % 2 == 0 ? "main" : "egg") + " of seat " + which / 2 + ") <- plan, " +
                            list.Count + " cards (was " + (__result?.Count ?? 0) + ")");
            __result = list;
        }

        // A reshuffle during the mulligans (a redraw): the plan's order again, so the new opening hand is the recorded one.
        static void ShuffleSourcesPostfix(ref Il2CppSystem.Collections.Generic.List<Il2Cpp.CardSource> __result)
        {
            try
            {
                if (_plan == null || __result == null || __result.Count == 0) return;
                var tsm = Il2Cpp.GManager.instance?.turnStateMachine;
                if (tsm == null || tsm.TurnCount > 0) return; // effect shuffles in play are left to DCGO
                int owner = __result[0].Owner.PlayerID;
                var want = _plan[owner * 2];
                var pool = new List<Il2Cpp.CardSource>();
                for (int i = 0; i < __result.Count; i++) pool.Add(__result[i]);
                if (pool.Count != want.Length) { MelonLogger.Warning("playback: reshuffle of " + pool.Count + " cards, plan has " + want.Length + " - left as is"); return; }
                var list = new Il2CppSystem.Collections.Generic.List<Il2Cpp.CardSource>();
                foreach (var id in want)
                {
                    int k = pool.FindIndex(c => c.CardID == id);
                    if (k < 0) { MelonLogger.Warning("playback: reshuffle lacks " + id + " - left as is"); return; }
                    list.Add(pool[k]);
                    pool.RemoveAt(k);
                }
                MelonLogger.Msg("playback: mulligan reshuffle of seat " + owner + " <- plan order");
                __result = list;
            }
            catch (Exception e) { MelonLogger.Warning("playback: reshuffle postfix: " + e.Message); }
        }

        static Assembly GameAssembly() =>
            AppDomain.CurrentDomain.GetAssemblies().FirstOrDefault(a => a.GetName().Name == "Assembly-CSharp");

        // Interop types keep the game's names but live under an "Il2Cpp" namespace prefix, so match by Name.
        static IEnumerable<Type> FindTypes(Assembly asm, string name)
        {
            Type[] all;
            try { all = asm.GetTypes(); }
            catch (ReflectionTypeLoadException e) { all = e.Types.Where(t => t != null).ToArray(); }
            return all.Where(t => t.Name == name);
        }

        void DumpSignatures()
        {
            var asm = GameAssembly();
            if (asm == null) { LoggerInstance.Error("Assembly-CSharp interop assembly not loaded"); return; }
            var sb = new StringBuilder();
            sb.AppendLine("# generated by DcgoRecorder " + DateTime.UtcNow.ToString("o"));
            const BindingFlags All = BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance
                | BindingFlags.Static | BindingFlags.DeclaredOnly;
            foreach (var name in DumpTypes)
            {
                var types = FindTypes(asm, name).ToList();
                if (types.Count == 0) { sb.AppendLine("## " + name + "  -- NOT FOUND"); continue; }
                foreach (var t in types)
                {
                    sb.AppendLine("## " + t.FullName + "  : " + string.Join(" -> ", BaseChain(t)));
                    Section(sb, "methods", () => t.GetMethods(All).OrderBy(m => m.Name, StringComparer.Ordinal)
                        .Select(m => (m.IsStatic ? "static " : "") + m.ReturnType.Name + " " + m.Name + "("
                            + string.Join(", ", m.GetParameters().Select(p => p.ParameterType.Name + " " + p.Name)) + ")"));
                    Section(sb, "fields", () => t.GetFields(All).OrderBy(f => f.Name, StringComparer.Ordinal)
                        .Select(f => (f.IsStatic ? "static " : "") + f.FieldType.Name + " " + f.Name));
                    Section(sb, "properties", () => t.GetProperties(All).OrderBy(f => f.Name, StringComparer.Ordinal)
                        .Select(f => f.PropertyType.Name + " " + f.Name));
                    Section(sb, "nested", () => t.GetNestedTypes(BindingFlags.Public | BindingFlags.NonPublic)
                        .Select(n => n.Name));
                }
            }
            // Every game type whose name suggests a hook site, so unexpected names can be found.
            sb.AppendLine("## ALL TYPES matching Select*/*Skill*/*Action*/*Effect*/Record*/Packet*");
            try
            {
                foreach (var t in SafeTypes(asm).Where(t => System.Text.RegularExpressions.Regex.IsMatch(t.Name,
                        "^(Select|Record|Packet)|Skill|Action|Effect$|Selection"))
                    .OrderBy(t => t.FullName, StringComparer.Ordinal))
                    sb.AppendLine("  " + t.FullName);
            }
            catch (Exception e) { sb.AppendLine("  (type scan failed: " + e.Message + ")"); }
            File.WriteAllText(Path.Combine(_dir, "signatures.txt"), sb.ToString());
            LoggerInstance.Msg("wrote signatures.txt (" + sb.Length + " chars)");
        }

        static IEnumerable<Type> SafeTypes(Assembly asm)
        {
            try { return asm.GetTypes(); }
            catch (ReflectionTypeLoadException e) { return e.Types.Where(t => t != null).ToArray(); }
        }

        static IEnumerable<string> BaseChain(Type t)
        {
            for (var b = t.BaseType; b != null; b = b.BaseType) yield return b.Name;
        }

        static void Section(StringBuilder sb, string label, Func<IEnumerable<string>> items)
        {
            try
            {
                var list = items().ToList();
                sb.AppendLine("  [" + label + " x" + list.Count + "]");
                foreach (var i in list) sb.AppendLine("    " + i);
            }
            catch (Exception e)
            {
                sb.AppendLine("  [" + label + " FAILED: " + e.GetType().Name + ": " + e.Message + "]");
            }
        }

        void ApplyProbes()
        {
            var asm = GameAssembly();
            if (asm == null) return;
            var postfix = new HarmonyMethod(typeof(Core).GetMethod(nameof(ProbePostfix), BindingFlags.Static | BindingFlags.NonPublic));
            int patched = 0;
            foreach (var (typeName, methodName) in Probes)
            {
                foreach (var t in FindTypes(asm, typeName))
                {
                    var methods = t.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance
                        | BindingFlags.Static | BindingFlags.DeclaredOnly).Where(m => m.Name == methodName).ToList();
                    foreach (var m in methods)
                    {
                        try { HarmonyInstance.Patch(m, postfix: postfix); patched++; LoggerInstance.Msg("probe patched " + t.Name + "." + m.Name); }
                        catch (Exception e) { LoggerInstance.Warning("could not patch " + t.Name + "." + m.Name + ": " + e.Message); }
                    }
                    if (methods.Count == 0) LoggerInstance.Warning("no method " + typeName + "." + methodName);
                }
            }
            try
            {
                var ctorPost = new HarmonyMethod(typeof(Core).GetMethod(nameof(PlayCtorPostfix), BindingFlags.Static | BindingFlags.NonPublic));
                foreach (var t in FindTypes(asm, "PlayCardClass"))
                    foreach (var pm in t.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.DeclaredOnly).Where(x => x.Name == "PlayCard"))
                    {
                        try { HarmonyInstance.Patch(pm, postfix: ctorPost); patched++; LoggerInstance.Msg("probe patched PlayCardClass.PlayCard"); }
                        catch (Exception e) { LoggerInstance.Warning("could not patch PlayCardClass.PlayCard: " + e.Message); }
                    }
            }
            catch (Exception e) { LoggerInstance.Warning("ctor probe failed: " + e.Message); }
            try
            {
                var countPost = new HarmonyMethod(typeof(Core).GetMethod(nameof(CountPostfix), BindingFlags.Static | BindingFlags.NonPublic));
                foreach (var t in FindTypes(asm, "SelectCountEffect"))
                    foreach (var m in t.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.DeclaredOnly).Where(x => x.Name == "SetCount"))
                    {
                        try { HarmonyInstance.Patch(m, postfix: countPost); patched++; LoggerInstance.Msg("probe patched SelectCountEffect.SetCount (typed)"); }
                        catch (Exception e) { LoggerInstance.Warning("could not patch SetCount: " + e.Message); }
                    }
            }
            catch (Exception e) { LoggerInstance.Warning("count probe failed: " + e.Message); }
            LoggerInstance.Msg("probes applied: " + patched);
            try
            {
                var names = asm.GetTypes().Select(t => t.FullName).Where(n => n != null && System.Text.RegularExpressions.Regex.IsMatch(n, "(^|[.])(AI|Bot|Auto|Heuristic|Simulat)|AIPlayer|AIAction|BotPlayer", System.Text.RegularExpressions.RegexOptions.IgnoreCase)).OrderBy(n => n).ToArray();
                File.WriteAllLines(Path.Combine(_dir, "types-ai.txt"), names);
                LoggerInstance.Msg("ai-ish types: " + names.Length);
            }
            catch (Exception e) { LoggerInstance.Warning("type list failed: " + e.Message); }
            // The wide call-logger wraps ~190 methods, including PlayCardClass checks the game calls deeply during
            // the bot's digivolve chains; the extra frames overflow the stack (System.StackOverflowException, WER
            // 0xc00000fd). Diagnostic only: enable with UserData/DcgoRecorder/wide.flag.
            if (File.Exists(Path.Combine(_dir, "wide.flag"))) ApplyWideProbes(asm);
        }

        // Diagnostic: log the first few calls of EVERY method of the turn state machine, to learn which of them the
        // game really reaches through its native call paths (the named probes above stayed silent for most).
        static readonly System.Collections.Concurrent.ConcurrentDictionary<string, int> _wideCounts = new System.Collections.Concurrent.ConcurrentDictionary<string, int>();

        void ApplyWideProbes(Assembly asm)
        {
            var lite = new HarmonyMethod(typeof(Core).GetMethod(nameof(LitePostfix), BindingFlags.Static | BindingFlags.NonPublic));
            int n = 0, bad = 0;
            foreach (var typeName in new[] { "TurnStateMachine", "SelectCardEffect", "SelectHandEffect", "SelectPermanentEffect",
                "SelectAttackEffect", "SelectCountEffect", "SelectDigiXrosClass", "UserSelectionManager", "MultipleSkills", "OptionalSkill",
                "PlayCardClass", "_PlayCard_d__45", "_BreedingPhase_d__28" })
                foreach (var t in FindTypes(asm, typeName))
                    foreach (var m in t.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance
                        | BindingFlags.Static | BindingFlags.DeclaredOnly))
                    {
                        if (m.IsSpecialName || m.IsGenericMethod || m.Name.StartsWith("get_") || m.Name.StartsWith("set_")) continue;
                        if (t.Name.StartsWith("_") && m.Name != "MoveNext") continue;
                        try { HarmonyInstance.Patch(m, postfix: lite); n++; }
                        catch { bad++; }
                    }
            LoggerInstance.Msg("wide probes: patched " + n + ", failed " + bad);
        }

        static void LitePostfix(MethodBase __originalMethod)
        {
            try
            {
                var key = (__originalMethod.DeclaringType?.Name ?? "?") + "." + __originalMethod.Name;
                int c = _wideCounts.AddOrUpdate(key, 1, (k, v) => v + 1);
                if (c > (__originalMethod.Name == "MoveNext" ? 12 : 3)) return;
                lock (Gate)
                    File.AppendAllText(Path.Combine(_dir, "wide-" + _session + ".txt"),
                        DateTime.UtcNow.ToString("HH:mm:ss.fff") + " " + key + " #" + c + Environment.NewLine);
            }
            catch { }
        }

        static Il2Cpp.CardSource ActiveCard(int index)
        {
            try
            {
                var gc = Il2Cpp.GManager.instance?.turnStateMachine?.gameContext;
                var list = gc?.ActiveCardList;
                if (list == null || index < 0 || index >= list.Count) return null;
                return list[index];
            }
            catch { return null; }
        }

        static object ReadMember(object o, string name)
        {
            try
            {
                var t = o.GetType();
                var pr = t.GetProperty(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                if (pr != null) return pr.GetValue(o);
                var f = t.GetField(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                return f?.GetValue(o);
            }
            catch { return null; }
        }

        // Extra fields for the selection setters: the picked cards' identities (the schema's `card_ids`),
        // and the prompt's own zone / mechanic tags where the class exposes them.
        static string SelectionExtras(object inst, string key, object[] args)
        {
            try
            {
                if ((key == "SelectCardEffect.SetTargetCardAndIndicies" || key == "SelectHandEffect.SetTargetHandCards") && args != null && args.Length >= 2)
                {
                    var sb = new StringBuilder(",\"card_ids\":[");
                    var arr = args[1] as System.Collections.IEnumerable;
                    bool first = true;
                    if (arr != null)
                        foreach (var o in arr)
                        {
                            var c = ActiveCard(Convert.ToInt32(o));
                            if (!first) sb.Append(',');
                            first = false;
                            sb.Append(c == null ? "null" : Quote(c.CardID));
                        }
                    sb.Append(']');
                    var sce = inst as Il2Cpp.SelectCardEffect;
                    if (sce != null)
                    {
                        // a card-panel answer takes a stamp too (both clients, in order): -2 marks a pick DCGO asked
                        // and the engine did not (identical candidates), which the converter drops
                        if (key == "SelectCardEffect.SetTargetCardAndIndicies" && Il2Cpp.GManager.instance?.You != null)
                        {
                            int spid = Convert.ToInt32(args[0]);
                            int sea;
                            if (spid == Il2Cpp.GManager.instance.You.PlayerID)
                            {
                                sea = _engineAction;
                                _engineAction = -1;
                                AppendStamp(spid, sea);
                            }
                            else sea = ReadStamp(spid, _remoteStampIdx[spid & 1]++);
                            if (sea == -2) sb.Append(",\"stray\":true");
                        }
                        sb.Append(",\"zone\":").Append(Quote(sce._root.ToString()));
                        if (sce._isAssembly) sb.Append(",\"mechanic\":\"assembly\"");
                        else if (sce._isDigiXros) sb.Append(",\"mechanic\":\"digixros\"");
                    }
                    return sb.ToString();
                }
            }
            catch { }
            try
            {
                // an ActivatePermanentAction's SkillIndex is POSITIONAL into the permanent's EffectList(OnDeclaration);
                // the engine's sub-slot is semantic (2 = [Main], 3 = <Link>): log which kind the index names
                if (key == "TurnStateMachine.QueueMainPhaseAction_Internal" && args != null && args.Length >= 3
                    && Convert.ToInt32(args[1]) == 2 && args[2] is System.Collections.IEnumerable pk)
                {
                    var bytes = new List<int>();
                    foreach (var o in pk) bytes.Add(Convert.ToInt32(o));
                    if (bytes.Count >= 8)
                    {
                        int be(int i) => (bytes[i] << 24) | (bytes[i + 1] << 16) | (bytes[i + 2] << 8) | bytes[i + 3];
                        int perm = be(0), skill = be(4);
                        var owner = Il2Cpp.GManager.instance?.GetPlayerFromID(Convert.ToInt32(args[0]));
                        var fp = owner?.GetFieldPermanents();
                        if (fp != null && perm >= 0 && perm < fp.Count)
                        {
                            var effs = fp[perm].EffectList(Il2Cpp.EffectTiming.OnDeclaration);
                            if (effs != null && skill >= 0 && skill < effs.Count && effs[skill] != null)
                            {
                                var nm = effs[skill].EffectName;
                                return ",\"activate_link\":" + ((nm != null && nm.StartsWith("Link (Cost:")) ? "true" : "false")
                                       + ",\"boards\":[" + PermIds(Il2Cpp.GManager.instance?.GetPlayerFromID(0)?.GetFieldPermanents()) + ","
                                       + PermIds(Il2Cpp.GManager.instance?.GetPlayerFromID(1)?.GetFieldPermanents()) + "]";
                            }
                        }
                    }
                }
            }
            catch { }
            try
            {
                if (key == "TurnStateMachine.QueueMainPhaseAction_Internal" && args != null && args.Length >= 3
                    && Convert.ToInt32(args[1]) == 4 && args[2] is System.Collections.IEnumerable raw)
                    return PlayPacketJson(Convert.ToInt32(args[0]), raw);
                // an attack packet's indices address GetFieldPermanents() of the turn player / the other seat: log that
                // order (both seats) so the replay can map them onto engine slots (remap_board_slots)
                if ((key == "TurnStateMachine.QueueMainPhaseAction_Internal" && args != null && args.Length >= 3
                     && (Convert.ToInt32(args[1]) == 0 || Convert.ToInt32(args[1]) == 2))
                    // a permanent pick's UnitIndex is a GetFieldPermanents() position too (DCGO's own order)
                    || key == "SelectPermanentEffect.SetTargetFrames")
                {
                    var gm = Il2Cpp.GManager.instance;
                    return ",\"boards\":[" + PermIds(gm?.GetPlayerFromID(0)?.GetFieldPermanents()) + "," +
                           PermIds(gm?.GetPlayerFromID(1)?.GetFieldPermanents()) + "]";
                }
            }
            catch { }
            try
            {
                // trigger order: DCGO's index is into ITS list of the pending effects; the card the local seat
                // picked in the panel names the effect, which is what the replay matches (chosen_effect_card)
                if ((key == "UserSelectionManager.SetBoolForPlayer" || key == "UserSelectionManager.SetIntForPlayer"
                     || key == "SelectCountEffect.SetCount" || key == "OptionalSkill.SetUseOptional") && args != null && args.Length >= 1
                    && Il2Cpp.GManager.instance?.You != null)
                {
                    // Every menu answer of a seat is stamped, in order, into that seat's stamp file (-1 = none), so
                    // the OTHER client - which sees the same answers in the same order over Photon - can attach the
                    // engine action to its copy of the row.
                    int pid = Convert.ToInt32(args[0]);
                    int ea;
                    if (pid == Il2Cpp.GManager.instance.You.PlayerID)
                    {
                        ea = _engineAction;
                        _engineAction = -1;
                        AppendStamp(pid, ea);
                    }
                    else ea = ReadStamp(pid, _remoteStampIdx[pid & 1]++);
                    return ea >= 0 ? ",\"engine_action\":" + ea : "";
                }
                if (key == "MultipleSkills.SetTargetSkill" && args != null && args.Length >= 2)
                {
                    var gm = Il2Cpp.GManager.instance;
                    var panel = gm?.selectCardPanel;
                    int pid = Convert.ToInt32(args[0]);
                    string chosen = null;
                    if (gm?.You != null && pid == gm.You.PlayerID)
                    {
                        if (panel != null && panel.SelectedList != null && panel.SelectedList.Count > 0 && panel.SelectedList[0] != null)
                            chosen = panel.SelectedList[0].CardID;
                        AppendChosen(pid, chosen);
                    }
                    else chosen = ReadChosen(pid, _remoteChosenIdx[pid & 1]++);
                    return chosen != null ? ",\"chosen_effect_card\":" + Quote(chosen) : "";
                }
                if (key == "TurnStateMachine.SetAttackingPermaent")
                {
                    // both battle areas, for the replay's slot remap (DCGO compact order -> engine play order)
                    var gm = Il2Cpp.GManager.instance;
                    var sb = new StringBuilder(",\"boards\":[");
                    for (int pid = 0; pid < 2; pid++)
                    {
                        if (pid > 0) sb.Append(',');
                        sb.Append(PermIds(gm?.GetPlayerFromID(pid)?.GetBattleAreaPermanents()));
                    }
                    return sb.Append(']').ToString();
                }
            }
            catch { }
            return "";
        }

        static Type _photonNetwork;

        static object PhotonStatic(string prop)
        {
            try
            {
                if (_photonNetwork == null)
                    foreach (var a in AppDomain.CurrentDomain.GetAssemblies())
                    {
                        Type[] ts;
                        try { ts = a.GetTypes(); } catch (ReflectionTypeLoadException e) { ts = e.Types; }
                        foreach (var t in ts)
                            if (t != null && t.Name == "PhotonNetwork" && t.GetProperty("InRoom") != null) { _photonNetwork = t; break; }
                        if (_photonNetwork != null) break;
                    }
                return _photonNetwork?.GetProperty(prop, BindingFlags.Public | BindingFlags.Static)?.GetValue(null);
            }
            catch { return null; }
        }

        // (inRoom, room name, player count) of the Photon client
        static (bool, string, int) PhotonRoom()
        {
            var inRoom = PhotonStatic("InRoom") as bool? ?? false;
            var room = PhotonStatic("CurrentRoom");
            string name = null; int count = 0;
            if (room != null)
            {
                try { name = room.GetType().GetProperty("Name")?.GetValue(room) as string; } catch { }
                try { count = Convert.ToInt32(room.GetType().GetProperty("PlayerCount")?.GetValue(room)); } catch { }
            }
            if (count == 0)
            {
                // Room.PlayerCount is not reachable by reflection on this build: count PhotonNetwork.PlayerList instead.
                try { if (PhotonStatic("PlayerList") is System.Collections.IEnumerable list) foreach (var _ in list) count++; } catch { }
            }
            return (inRoom, name, count);
        }

        // PlayCardAction wire format (Photon Protocol, big-endian): cardIndex, targetFrameID, then the jogress /
        // burst / app-fusion operands. Resolved here, while the card is still in hand, to the fields the converter
        // needs: hand slot, digivolve target (or a new frame), both boards.
        static string PlayPacketJson(int playerId, System.Collections.IEnumerable raw)
        {
            var bytes = new List<byte>();
            foreach (var o in raw) bytes.Add(Convert.ToByte(o));
            if (bytes.Count < 8) return "";
            int ReadInt(int at) => (bytes[at] << 24) | (bytes[at + 1] << 16) | (bytes[at + 2] << 8) | bytes[at + 3];
            int cardIndex = ReadInt(0), frame = ReadInt(4);
            var gm = Il2Cpp.GManager.instance;
            var pl = gm?.GetPlayerFromID(playerId);
            var sb = new StringBuilder(",\"play\":{\"card_index\":" + cardIndex + ",\"frame\":" + frame);
            if (pl != null)
            {
                int slot = -1;
                string id = null;
                for (int i = 0; i < pl.HandCards.Count; i++)
                    if (pl.HandCards[i] != null && pl.HandCards[i].CardIndex == cardIndex) { slot = i; id = pl.HandCards[i].CardID; break; }
                sb.Append(",\"hand_slot\":").Append(slot).Append(",\"card_id\":").Append(id == null ? "null" : Quote(id));
                var frames = pl.fieldCardFrames;
                bool breeding = frames != null && frame == frames.Count - 1;
                Il2Cpp.Permanent target = null;
                if (breeding)
                {
                    // the breeding frame reports IsEmptyFrame() even with an egg in it: ask the breeding area directly
                    var br = pl.GetBreedingAreaPermanents();
                    target = br != null && br.Count > 0 ? br[0] : null;
                }
                else if (frames != null && frame >= 0 && frame < frames.Count && !frames[frame].IsEmptyFrame())
                {
                    {
                        var bt = pl.GetBattleAreaPermanents();
                        for (int i = 0; i < bt.Count; i++)
                            if (bt[i] != null && frames.IndexOf(bt[i].PermanentFrame) == frame) { target = bt[i]; sb.Append(",\"target_slot\":").Append(i); break; }
                    }
                }
                sb.Append(",\"target_breeding\":").Append(breeding && target != null ? "true" : "false");
                sb.Append(",\"target\":").Append(target == null ? "null" : CardJson(target.TopCard));
            }
            sb.Append("},\"boards\":[");
            for (int pid = 0; pid < 2; pid++)
            {
                if (pid > 0) sb.Append(',');
                sb.Append(PermIds(gm?.GetPlayerFromID(pid)?.GetBattleAreaPermanents()));
            }
            return sb.Append(']').ToString();
        }

        static int TurnPlayerId()
        {
            try
            {
                var gc = Il2Cpp.GManager.instance?.turnStateMachine?.gameContext;
                if (gc == null) return -1;
                var tp = gc.TurnPlayer;
                return tp == null ? -1 : tp.PlayerID;
            }
            catch { return -1; }
        }

        static int CurrentTurn()
        {
            try { return Il2Cpp.GManager.instance?.turnStateMachine?.TurnCount ?? -1; } catch { return -1; }
        }

        static string CardJson(Il2Cpp.CardSource c)
        {
            if (c == null) return "null";
            return "{\"card_id\":" + Quote(c.CardID) + ",\"card_index\":" + c.CardIndex + "}";
        }

        // The hand of the seat that owns the card being played, with each card's activeCardIndex, in hand order:
        // the Unity recorder encodes a play as the card's slot in this list (ActionEncoder.EncodePlayCard).
        static string OwnerHandJson(Il2Cpp.PlayCardClass pc)
        {
            try
            {
                var gm = Il2Cpp.GManager.instance;
                var first = pc?.CardSources != null && pc.CardSources.Count > 0 ? pc.CardSources[0] : null;
                if (gm == null || first == null) return "null";
                for (int pid = 0; pid < 2; pid++)
                {
                    var pl = gm.GetPlayerFromID(pid);
                    if (pl?.HandCards == null) continue;
                    for (int i = 0; i < pl.HandCards.Count; i++)
                        if (pl.HandCards[i] != null && pl.HandCards[i].CardIndex == first.CardIndex)
                        {
                            var sb = new StringBuilder("{\"player\":" + pid + ",\"slot\":" + i + ",\"cards\":[");
                            for (int j = 0; j < pl.HandCards.Count; j++)
                            {
                                if (j > 0) sb.Append(',');
                                sb.Append(CardJson(pl.HandCards[j]));
                            }
                            return sb.Append("]}").ToString();
                        }
                }
                return "{\"player\":-1,\"slot\":-1}";
            }
            catch { return "null"; }
        }

        // Where a digivolution lands, in the engine's action-space terms: the compact battle-area position of the
        // target (GetBattleAreaPermanents order) or the breeding area (-> BREEDING_TARGET, 14), plus both seats' battle
        // areas as card ids so the replay can remap slots by identity (it does so from the action row's board lists).
        static string TargetSlotJson(Il2Cpp.PlayCardClass pc)
        {
            try
            {
                var t = pc?._targetPermanent;
                var gm = Il2Cpp.GManager.instance;
                if (t == null || gm == null) return "";
                var sb = new StringBuilder();
                int pid = TurnPlayerId();
                var me = gm.GetPlayerFromID(pid);
                int slot = -1;
                bool breeding = false;
                var br = me?.GetBreedingAreaPermanents();
                if (br != null)
                    for (int i = 0; i < br.Count; i++)
                        if (br[i] != null && br[i].Pointer == t.Pointer) breeding = true;
                var bt = me?.GetBattleAreaPermanents();
                if (bt != null)
                    for (int i = 0; i < bt.Count; i++)
                        if (bt[i] != null && bt[i].Pointer == t.Pointer) slot = i;
                sb.Append(",\"target_slot\":").Append(slot).Append(",\"target_breeding\":").Append(breeding ? "true" : "false");
                sb.Append(",\"boards\":[");
                for (int p = 0; p < 2; p++)
                {
                    if (p > 0) sb.Append(',');
                    sb.Append(PermIds(gm.GetPlayerFromID(p)?.GetBattleAreaPermanents()));
                }
                return sb.Append(']').ToString();
            }
            catch { return ""; }
        }

        static void PlayCtorPostfix(object __instance)
        {
            try
            {
                var pc = __instance as Il2Cpp.PlayCardClass;
                var sb = new StringBuilder();
                sb.Append("{\"seq\":").Append(System.Threading.Interlocked.Increment(ref _seq))
                  .Append(",\"t\":\"").Append(DateTime.UtcNow.ToString("o"))
                  .Append("\",\"method\":\"PlayCardClass.PlayCard\",\"cards\":").Append(pc == null ? "null" : Ids(pc.CardSources))
                  .Append(",\"pay_cost\":").Append(pc != null && pc.PayCost ? "true" : "false")
                  .Append(",\"breeding_area\":").Append(pc != null && pc._isBreedingArea ? "true" : "false")
                  .Append(",\"target\":").Append(pc == null || pc._targetPermanent == null ? "null" : CardJson(pc._targetPermanent.TopCard))
                  .Append(",\"hand\":").Append(OwnerHandJson(pc))
                  .Append(TargetSlotJson(pc))
                  .Append(",\"tp\":").Append(TurnPlayerId()).Append(",\"turn\":").Append(CurrentTurn()).Append('}');
                lock (Gate)
                    File.AppendAllText(Path.Combine(_dir, "raw-" + _session + ".jsonl"), sb.ToString() + "\n");
            }
            catch { }
        }

        static void CountPostfix(int __0, int __1)
        {
            try
            {
                var sb = new StringBuilder();
                sb.Append("{\"seq\":").Append(System.Threading.Interlocked.Increment(ref _seq))
                  .Append(",\"t\":\"").Append(DateTime.UtcNow.ToString("o"))
                  .Append("\",\"method\":\"SelectCountEffect.SetCount\",\"args\":[").Append(__0).Append(',').Append(__1)
                  .Append("],\"tp\":").Append(TurnPlayerId()).Append(",\"turn\":").Append(CurrentTurn())
                  // the engine action this count realizes (a cost menu shows DCGO's unreduced costs, so the value
                  // alone cannot name the engine's route)
                  .Append(SelectionExtras(null, "SelectCountEffect.SetCount", new object[] { __0, __1 })).Append('}');
                lock (Gate)
                    File.AppendAllText(Path.Combine(_dir, "raw-" + _session + ".jsonl"), sb.ToString() + "\n");
            }
            catch { }
        }

        static int _snapshots;
        static int _redrawSnapshots; // pre-mulligan (SetRedraw) snapshots written in the current game
        static bool _gameEnded;

        // ---- agent seat: one of our trained models plays the local seat (docs/DCGO_MELONLOADER.md) ------------------
        // Opt-in by UserData/DcgoRecorder/agent.txt (one line: the request directory shared with
        // `digimon-engine-cli dcgo-agent --dir <dir>`). At each local decision point the mod writes a fresh snapshot
        // row, then request-<n>.json; the agent replays the game so far and answers; the mod performs the answer
        // through the same calls a human click reaches (so the recorder logs it like any other decision).
        string _agentDir;          // null = not read, "" = off
        int _agentSeq;
        string _agentPending;      // id of the request in flight
        float _agentNextAt;
        long _agentRowsAtAct = -1; // raw-log length when the last action was performed: wait for it to change

        static long RawLength()
        {
            try { var f = new FileInfo(Path.Combine(_dir, "raw-" + _session + ".jsonl")); return f.Exists ? f.Length : 0; }
            catch { return 0; }
        }

        void AgentStep(float now)
        {
            if (_agentDir == null)
            {
                var cfg = Path.Combine(_dir ?? "", "agent.txt");
                _agentDir = File.Exists(cfg) ? File.ReadAllText(cfg).Trim() : "";
                if (_agentDir != "") LoggerInstance.Msg("agent: model seat ON, requests in " + _agentDir);
            }
            if (_agentDir == "" || now < _agentNextAt) return;
            _agentNextAt = now + 0.5f;
            try
            {
                var gm = Il2Cpp.GManager.instance;
                var tsm = gm != null ? gm.turnStateMachine : null;
                if (gm == null || tsm == null || gm.You == null) return;
                int you = gm.You.PlayerID;

                if (_agentPending != null)
                {
                    var resp = Path.Combine(_agentDir, "response-" + _agentPending + ".json");
                    if (!File.Exists(resp))
                    {
                        if (now - _agentPendingAt > 45f)
                        {
                            LoggerInstance.Warning("agent: no answer to request " + _agentPending + " in 45 s - asking again");
                            _agentPending = null;
                        }
                        return;
                    }
                    var text = File.ReadAllText(resp);
                    File.Delete(resp);
                    _agentPending = null;
                    Perform(text, gm, tsm, now);
                    return;
                }

                // a decision point for the local seat: its turn (or the mulligan), no effect prompt open, and the game
                // has moved on since the last action we performed
                bool mulligan = FindActiveButton("NotSelectButton") != null && tsm.TurnCount == 0;
                if (!mulligan && tsm.TurnCount < 1) return; // turn 0 after the mulligans: TurnPlayer is still stale
                if (!mulligan && TurnPlayerId() != you && !tsm.IsSelecting) return;
                long len = RawLength();
                if (len == _agentRowsAtAct && now < _agentRetryAt) return;

                WriteSnapshot("Request");
                var id = (++_agentSeq).ToString("D4") + "-" + _session;
                var req = "{\"id\":" + Quote(id) + ",\"raw\":" + Quote(Path.Combine(_dir, "raw-" + _session + ".jsonl").Replace("\\", "/")) + ",\"seat\":" + you + "}";
                var tmp = Path.Combine(_agentDir, "request-" + id + ".json.tmp");
                File.WriteAllText(tmp, req);
                File.Move(tmp, Path.Combine(_agentDir, "request-" + id + ".json"));
                _agentPending = id;
                _agentPendingAt = now;
            }
            catch (Exception e)
            {
                // not performed (a button not ready yet, a card not found): the same answer next time is a retry,
                // not a repeat, so it must not trip the no-progress guard
                LoggerInstance.Warning("agent: " + e.Message.Split('\n')[0]);
                _agentLastSig = null;
                _agentNextAt = now + 3f;
            }
        }

        float _agentRetryAt;
        float _agentPendingAt;
        string _agentLastSig;
        float _agentLastSigAt;
        float _panelOpenSince = float.MaxValue;
        string _cardWaitReq = null;
        float _cardWaitSince;
        bool _agentResent;
        bool _agentStallWarned;

        // DCGO only dequeues a main-phase action in MainPhase()'s wait loop (Main phase, no play / attack / effect in
        // progress), and resolves its indices against the board AT DEQUEUE time. Queueing earlier (during a breeding
        // move or an attack still resolving) risks an index that names another permanent or none, which DCGO drops
        // silently. Breeding answers are only sent in the Breeding phase.
        static bool DcgoReadyForTurnAction(Il2Cpp.TurnStateMachine tsm, Il2Cpp.Player me)
        {
            try
            {
                var phase = tsm.gameContext.TurnPhase.ToString();
                if (phase == "Breeding") return true;
                if (phase != "Main") return false;
                if (Member(tsm, "AttackingPermanent") != null || Member(tsm, "PlayCard") != null || Member(tsm, "UseCardEffect") != null) return false;
                return !me.HasMainPhaseAction();
            }
            catch { return true; } // a build without these members: fall back to the old behaviour
        }

        static object Member(object o, string name)
        {
            var t = o.GetType();
            var p = t.GetProperty(name, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic);
            if (p != null) return p.GetValue(o);
            var m = t.GetMethod("get_" + name, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic, null, Type.EmptyTypes, null);
            return m?.Invoke(o, null);
        }

        static UnityEngine.UI.Button FindActiveButton(string name)
        {
            foreach (var b in UnityEngine.Object.FindObjectsOfType<UnityEngine.UI.Button>())
                if (b != null && b.name == name && b.gameObject.activeInHierarchy && b.interactable) return b;
            return null;
        }

        static string Field(string json, string key)
        {
            // tiny reader for the agent's flat response (string, number, bool or null values)
            var k = "\"" + key + "\":";
            int i = json.IndexOf(k, StringComparison.Ordinal);
            if (i < 0) return null;
            i += k.Length;
            while (i < json.Length && json[i] == ' ') i++;
            if (i >= json.Length) return null;
            if (json[i] == '"')
            {
                int j = json.IndexOf('"', i + 1);
                return j < 0 ? null : json.Substring(i + 1, j - i - 1);
            }
            int e = i;
            while (e < json.Length && ",}\n\r ".IndexOf(json[e]) < 0) e++;
            var v = json.Substring(i, e - i);
            return v == "null" ? null : v;
        }

        // The picks of a selection response, in order: (pick, card_id, owner, slot, index) per entry.
        static List<Dictionary<string, string>> Picks(string json)
        {
            var list = new List<Dictionary<string, string>>();
            // the agent writes pretty-printed JSON ("picks": [ ... ]): find the key, then its bracket
            int i = json.IndexOf("\"picks\"", StringComparison.Ordinal);
            if (i < 0) return list;
            i = json.IndexOf('[', i);
            if (i < 0) return list;
            // the closing bracket, skipping quoted strings (labels and prompts contain "[Greymon]"-style text)
            int end = -1;
            bool quoted = false;
            for (int j = i + 1; j < json.Length && end < 0; j++)
            {
                if (json[j] == '\\' && quoted) { j++; continue; }
                if (json[j] == '"') quoted = !quoted;
                else if (!quoted && json[j] == ']') end = j;
            }
            if (end < 0) return list;
            var body = json.Substring(i, end - i);
            foreach (var part in body.Split('{'))
            {
                if (!part.Contains("\"pick\"")) continue;
                var obj = "{" + part;
                var d = new Dictionary<string, string>();
                foreach (var k in new[] { "pick", "card_id", "owner", "slot", "index", "zone", "label", "carrier_card_id" }) d[k] = Field(obj, k);
                list.Add(d);
            }
            return list;
        }

        void AnswerSelection(string text, Il2Cpp.GManager gm, Il2Cpp.TurnStateMachine tsm, float now)
        {
            var category = Field(text, "category");
            var picks = Picks(text);
            int you = gm.You.PlayerID;
            // the engine action this answer realizes: stamped on DCGO's own answer row (bool / int / count menus),
            // whose encoding need not follow the engine's branch order
            _engineAction = (category == "effect_choice" || category == "optional") && int.TryParse(Field(text, "action_id") ?? "", out var ea) ? ea : -1;
            bool declined = picks.Count == 0 || picks.TrueForAll(p => p["pick"] == "decline");
            var chosen = picks.FindAll(p => p["pick"] != "decline");
            LoggerInstance.Msg("agent: answering " + category + " prompt '" + Field(text, "prompt") + "' with " +
                               (declined ? "no pick" : string.Join(", ", chosen.ConvertAll(p => p["card_id"] ?? p["index"] ?? "?"))));
            // DCGO may ask which of several triggers to resolve first where the engine had only one queued (its
            // condition already false, e.g. "if your hand has 7 or fewer"): the panel then lists trigger SOURCE cards,
            // none of them our picks. Resolve the engine's effect first (its source card), then answer the prompt.
            var src = Field(text, "source_card_id");
            var opanel = gm.selectCardPanel;
            if (DeclineStrayAssembly(gm, Field(text, "prompt"))) throw new Exception("assembly element declined - the prompt follows");
            if (category != "trigger_order" && !string.IsNullOrEmpty(src) && opanel != null && opanel.gameObject.activeInHierarchy)
            {
                var ids = opanel.GetComponentsInChildren<Il2Cpp.HandCard>(false).Select(h => h?.cardSource?.CardID).ToList();
                // The engine DECLINES its effect (an optional gate answered PASS: "Use Cost reduction ...?") while DCGO
                // lists it among several in its "Multiple effects" panel: that is DCGO's "Don't activate these
                // effects" (No Selection, -1), not resolving it first (ST23-15 played with two BT25-049 reducers).
                if (declined && ids.Contains(src) && PanelSays(opanel, "Multiple effects") && FindActiveButton("NotSelectButton") != null)
                {
                    LoggerInstance.Msg("agent: DCGO lists the effect the engine declines among others - Don't activate these effects");
                    opanel.OnClickNotSelectButton();
                    throw new Exception("effects declined - the prompt follows");
                }
                if (ids.Contains(src) && !chosen.Any(c => c["card_id"] != null && ids.Contains(c["card_id"])))
                {
                    LoggerInstance.Msg("agent: DCGO asks the order of triggers the engine did not order - resolving " + src + " first");
                    DriveCardPanel(opanel, new List<Dictionary<string, string>> { new Dictionary<string, string> { ["card_id"] = src } }, false);
                    throw new Exception("trigger order answered - the prompt follows");
                }
            }
            switch (category)
            {
                case "optional":
                    {
                        // DCGO's OptionalSkill: a "Use" button and the side-bar back button (don't use)
                        if (!MenuOpen()) throw new Exception("optional-effect prompt not open yet");
                        if (declined)
                        {
                            var back = FindActiveButtonPath("戻るボタン");
                            if (back != null) { LoggerInstance.Msg("agent: optional effect: not used (back button)"); back.onClick.Invoke(); break; }
                            if (ClickMenu(t => t.StartsWith("no") || t.StartsWith("don") || t.Contains("not "))) break;
                            throw new Exception("optional prompt has no decline");
                        }
                        if (!ClickMenu(t => t == "use" || t.StartsWith("yes") || t.StartsWith("activate")))
                            throw new Exception("optional prompt has no Use button");
                        break;
                    }
                case "trigger_order":
                    {
                        // DCGO's MultipleSkills prompt: the card panel lists each pending trigger's source card
                        var tpanel = gm.selectCardPanel;
                        if (tpanel == null || !tpanel.gameObject.activeInHierarchy)
                        {
                            // ONE trigger left: DCGO resolves a lone effect without the panel (MultipleSkills
                            // `skillInfos_active.Count == 1`) and asks only its "Use?" - which is the engine's order
                            // pick when every queued trigger is optional (BT24-083 at the start of the turn): "Use"
                            // realizes the pick, the back button the engine's decline-all
                            if (int.TryParse(Field(text, "action_id") ?? "", out var toa)
                                && ConfirmOptional(declined, declined ? -1 : toa)) break;
                            throw new Exception("trigger-order panel not open yet");
                        }
                        DriveCardPanel(tpanel, chosen, declined);
                        break;
                    }
                case "card":
                    {
                        // For the local seat DCGO opens its card-selection panel and waits for the human to finish it
                        // (SelectCardEffect -> SelectCardPanel.OpenSelectCardPanel); only then does it send the answer.
                        // A setter call alone is queued but never read while the panel is open, so click the panel.
                        var panel = gm.selectCardPanel;
                        if (panel != null && panel.gameObject.activeInHierarchy)
                        {
                            DriveCardPanel(panel, chosen, declined);
                            break;
                        }
                        // DCGO may first ask WHERE from (a command menu: "From Hand" / "From Trash" / "Don't place any",
                        // EX12-018) where the engine asks one pick over both zones: answer the menu with the zone of the
                        // pick, then retry - the pick itself comes in the panel / hand prompt DCGO opens next.
                        if (ConfirmOptional(declined)) break;
                        // the carrier of a source pick is clicked; DCGO then wants its End Selection (the pick follows)
                        if (!declined && chosen.Count > 0 && chosen[0]["carrier_card_id"] != null)
                        {
                            var only = MenuButtons().Select(b => ButtonText(b).Trim().ToLowerInvariant()).ToList();
                            if (only.Count == 1 && only[0] == "end selection")
                            {
                                ClickMenu(t => t == "end selection");
                                throw new Exception("carrier confirmed - the source pick follows");
                            }
                        }
                        if (MenuOpen())
                        {
                            var zone = declined ? null : chosen[0]["zone"];
                            // a decline is "Don't ..." / "No Selection", or ending the selection with nothing picked.
                            // Stamp it PASS: the int row it leaves is then the engine's decline on its own, where an
                            // unstamped zone int ("From trash") waits for the card pick that follows it
                            if (declined) _engineAction = 62;
                            bool clicked = declined
                                ? ClickMenu(t => t.StartsWith("don") || t.StartsWith("no") || t.Contains("not ") || t == "end selection")
                                : ClickMenu(t => zone != null && t.Contains(zone.ToLowerInvariant()));
                            if (!clicked) _engineAction = -1;
                            if (clicked)
                            {
                                if (declined) break;
                                throw new Exception("zone menu answered (" + zone + ") - the pick follows");
                            }
                            // not a zone menu: DCGO's hand prompt shows only "End Selection" here, and it reads its
                            // answer queue (SelectHandEffect waits on HasPlayerSelection), so the hand RPC answers it
                            if ((declined || chosen.TrueForAll(c => c["zone"] == "Hand"))
                                && UnityEngine.Object.FindObjectOfType<Il2Cpp.SelectHandEffect>() != null)
                            {
                                // a decline is an empty pick (P-248's start-of-main declined this way)
                                AnswerHandPick(gm, you, declined ? new List<Dictionary<string, string>>() : chosen);
                                break;
                            }
                            throw new Exception("menu has no option for zone " + (zone ?? "decline"));
                        }
                        // a pick in the hand is DCGO's SelectHandEffect, which reads its answer queue directly
                        if (!declined && chosen.TrueForAll(c => c["zone"] == "Hand") && UnityEngine.Object.FindObjectOfType<Il2Cpp.SelectHandEffect>() != null
                            && tsm.IsSelecting)
                        {
                            AnswerHandPick(gm, you, chosen);
                            break;
                        }
                        // A digivolution-source pick: DCGO first asks which Digimon holds it (field cards clickable),
                        // then shows its sources in the panel. Click the carrier; the source follows in the panel.
                        var carrierId = chosen.Count > 0 ? chosen[0]["carrier_card_id"] : null;
                        if (!declined && carrierId != null && FieldClickable())
                        {
                            foreach (var f in UnityEngine.Object.FindObjectsOfType<Il2Cpp.FieldPermanentCard>())
                            {
                                var top = f?.ThisPermanent?.TopCard;
                                if (f == null || f.OnClickAction == null || top == null || top.CardID != carrierId || top.Owner?.PlayerID != you) continue;
                                LoggerInstance.Msg("agent: source carrier click " + carrierId);
                                f.OnClickAction.Invoke(f);
                                throw new Exception("carrier chosen - the source pick follows");
                            }
                        }
                        // Not open yet (a reveal is still animating): retry. Never answer through the setter here - an
                        // early answer sits in DCGO's selection queue and is consumed by a LATER prompt.
                        if (TurnPlayerId() == you || gm.IsAI) throw new Exception("card selection panel not open yet");
                        // the non-turn seat gets DCGO's panel too, a moment later (EX12-074's [Security] during the
                        // opponent's attack): an answer through the setter before it opens is recorded but leaves the
                        // panel waiting. Give it time; use the setter only when no panel comes.
                        var reqId = (Field(text, "prompt") ?? "") + "@" + (Field(text, "steps") ?? "");
                        if (_cardWaitReq != reqId) { _cardWaitReq = reqId; _cardWaitSince = now; }
                        if (now - _cardWaitSince < 10f) throw new Exception("card selection panel not open yet (non-turn seat)");
                        var sce = UnityEngine.Object.FindObjectOfType<Il2Cpp.SelectCardEffect>();
                        if (sce == null) throw new Exception("no SelectCardEffect open");
                        var roots = sce.RootCardList();
                        var used = new HashSet<int>();
                        var idx = new List<int>();
                        foreach (var pk in chosen)
                            for (int i = 0; roots != null && i < roots.Count; i++)
                                if (roots[i] != null && roots[i].CardID == pk["card_id"] && used.Add(roots[i].CardIndex)) { idx.Add(roots[i].CardIndex); break; }
                        if (idx.Count != chosen.Count) throw new Exception("picked card not among the prompt's candidates");
                        if (gm.IsAI) sce.SetTargetCardAndIndicies(you, idx.ToArray(), null);
                        else Rpc(sce, "SetTargetCardAndIndicies", Box(you), Box(idx.ToArray()), null);
                        break;
                    }
                case "hand":
                    {
                        // The local seat's hand prompt is UI (hand cards clickable, "End Selection" in the side bar,
                        // "No Selection" on the back button). Answer it through that UI once it is open - an RPC sent
                        // earlier is queued and read by a LATER prompt, and DCGO then plays the turn without it.
                        // Some effects pick from the hand in DCGO's card panel (SelectCardEffect over the hand cards)
                        // rather than with clickable hand cards: answer the panel then.
                        var hpanel = gm.selectCardPanel;
                        if (hpanel != null && hpanel.gameObject.activeInHierarchy)
                        {
                            DriveCardPanel(hpanel, chosen, declined);
                            break;
                        }
                        bool backOpen = FindActiveButtonPath("戻るボタン") != null;
                        if (!tsm.IsSelecting || !(MenuOpen() || backOpen || HandClickable(gm))) throw new Exception("hand prompt not open yet");
                        if (declined)
                        {
                            if (ClickMenu(t => t.StartsWith("no selection") || t.StartsWith("don") || t == "end selection")) break;
                            var back = FindActiveButtonPath("戻るボタン");
                            if (back == null) throw new Exception("hand prompt has no way to decline");
                            LoggerInstance.Msg("agent: hand prompt: No Selection (back button)");
                            // when the back button is an optional effect's "don't use", DCGO answers SetUseOptional:
                            // stamp the engine's decline on it (a hand "No Selection" logs no stamped row)
                            _engineAction = 62;
                            back.onClick.Invoke();
                            break;
                        }
                        AnswerHandPick(gm, you, chosen);
                        break;
#pragma warning disable CS0162
                        var she = UnityEngine.Object.FindObjectOfType<Il2Cpp.SelectHandEffect>();
                        if (she == null) throw new Exception("no SelectHandEffect open");
                        var used = new HashSet<int>();
                        var idx = new List<int>();
                        foreach (var pk in chosen)
                        {
                            var owner = gm.GetPlayerFromID(int.Parse(pk["owner"] ?? you.ToString()));
                            for (int i = 0; i < owner.HandCards.Count; i++)
                                if (owner.HandCards[i]?.CardID == pk["card_id"] && used.Add(owner.HandCards[i].CardIndex)) { idx.Add(owner.HandCards[i].CardIndex); break; }
                        }
                        if (idx.Count != chosen.Count) throw new Exception("picked hand card not found");
                        if (gm.IsAI) she.SetTargetHandCards(you, idx.ToArray());
                        else Rpc(she, "SetTargetHandCards", Box(you), Box(idx.ToArray()));
                        break;
                    }
                case "permanent":
                    {
                        var spe = UnityEngine.Object.FindObjectOfType<Il2Cpp.SelectPermanentEffect>();
                        if (spe == null) throw new Exception("no SelectPermanentEffect open");
                        // Only answer a prompt DCGO actually opened (candidates carry a click target). A forced pick
                        // (mandatory, candidates == max) DCGO resolves by itself while the engine still asks: an RPC
                        // then would be an extra answer that a LATER prompt consumes.
                        int selfPick = -1;
                        var srcId = Field(text, "source_card_id");
                        if (!declined && chosen.Count == 1 && srcId != null && chosen[0]["card_id"] == srcId
                            && int.TryParse(Field(text, "action_id") ?? "", out var selfAct))
                            selfPick = selfAct;
                        if (!gm.IsAI && !FieldClickable() && ConfirmOptional(declined, selfPick)) break;
                        if (!gm.IsAI && !FieldClickable())
                            throw new Exception("permanent prompt not open (DCGO may have resolved a forced pick itself)");
                        var isTurn = new List<bool>();
                        var unit = new List<int>();
                        int tp = TurnPlayerId();
                        foreach (var pk in chosen)
                        {
                            int ownerId = int.Parse(pk["owner"] ?? "-1");
                            var owner = gm.GetPlayerFromID(ownerId);
                            var perm = FieldByCard(owner, pk["card_id"], int.Parse(pk["slot"] ?? "-1"));
                            if (perm == null) throw new Exception("picked permanent " + pk["card_id"] + " not found");
                            isTurn.Add(ownerId == tp);
                            unit.Add(owner.GetFieldPermanents().IndexOf(perm));
                        }
                        if (gm.IsAI) spe.SetTargetFrames(you, isTurn.ToArray(), unit.ToArray());
                        else Rpc(spe, "SetTargetFrames", Box(you), Box(isTurn.ToArray()), Box(unit.ToArray()));
                        break;
                    }
                case "effect_choice":
                    {
                        // the engine's digivolution-cost choice is DCGO's SelectCountEffect over the candidate COSTS
                        // ("Which digivolution cost do you pay?", CardController): answer with the cost, not the index
                        var label = chosen.Count > 0 ? chosen[0]["label"] : null;
                        // the engine's replacement order ("Choose the next immediate replacement effect") is DCGO's
                        // MultipleSkills card panel ("Multiple effects are triggered"), listing each effect's source card
                        var mp = gm.selectCardPanel;
                        var bsrc = chosen.Count > 0 ? chosen[0]["card_id"] : null;
                        if (!string.IsNullOrEmpty(bsrc) && mp != null && mp.gameObject.activeInHierarchy && PanelSays(mp, "Multiple effects"))
                        {
                            LoggerInstance.Msg("agent: effect order: resolving " + bsrc + " first for '" + (label ?? "?") + "'");
                            DriveCardPanel(mp, new List<Dictionary<string, string>> { new Dictionary<string, string> { ["card_id"] = bsrc } }, false);
                            break;
                        }
                        if (MenuOpen() && label != null)
                        {
                            var want = label.ToLowerInvariant();
                            var first = want.Split(' ')[0];
                            if (ClickMenu(t => t == want || t.Contains(want) || want.Contains(t) || t.StartsWith(first)))
                                break;
                        }
                        // The local seat always gets DCGO's own menu for these (cost menu, zone menu, top/bottom, ...):
                        // wait for it. An answer sent before the menu opens is queued and read by a LATER prompt.
                        if (!MenuOpen()) throw new Exception("menu not open yet for '" + (label ?? "?") + "'");
                        // wording differs ("1 opponent Digimon gets -13000 DP" vs DCGO's "DP - 13K"): both list the
                        // branches in card-text order, so fall back to the engine branch's position
                        if (chosen.Count > 0 && int.TryParse(chosen[0]["index"], out var pos))
                        {
                            var buttons = MenuButtons();
                            if (pos >= 0 && pos < buttons.Count)
                            {
                                LoggerInstance.Msg("agent: menu: by position " + pos + " '" + ButtonText(buttons[pos]) + "' for '" + label + "'");
                                buttons[pos].onClick.Invoke();
                                break;
                            }
                        }
                        throw new Exception("menu open but no option matches '" + (label ?? "?") + "'");
                        const string costPrefix = "Digivolve for cost ";
                        if (label != null && label.StartsWith(costPrefix))
                        {
                            var digits = new string(label.Substring(costPrefix.Length).TakeWhile(char.IsDigit).ToArray());
                            int cost = int.Parse(digits);
                            var sce2 = UnityEngine.Object.FindObjectOfType<Il2Cpp.SelectCountEffect>();
                            if (sce2 == null) throw new Exception("no SelectCountEffect open for the digivolution cost");
                            LoggerInstance.Msg("agent: digivolution cost " + cost);
                            if (gm.IsAI) sce2.SetCount(you, cost);
                            else Rpc(sce2, "SetCount", Box(you), Box(cost));
                            break;
                        }
                        var usm = UnityEngine.Object.FindObjectOfType<Il2Cpp.UserSelectionManager>();
                        if (usm == null) throw new Exception("no UserSelectionManager");
                        int index = chosen.Count > 0 ? int.Parse(chosen[0]["index"] ?? "0") : 0;
                        if (gm.IsAI) usm.SetIntForPlayer(you, index);
                        else Rpc(usm, "SetIntForPlayer", Box(you), Box(index));
                        break;
                    }
                default:
                    LoggerInstance.Warning("agent: selection category '" + category + "' is not answered yet - leaving it to the human");
                    _agentRetryAt = now + 30f;
                    break;
            }
        }

        // PvP: a selection answer has to reach BOTH clients. DCGO's own UI sends it as
        // photonView.RPC(<setter>, RpcTarget.All, ...) (SelectCardEffect / SelectHandEffect / SelectPermanentEffect /
        // UserSelectionManager); a local setter call would resolve the prompt on this client only and desync the match.
        static void Rpc(UnityEngine.Component c, string method, params Il2CppSystem.Object[] args)
        {
            var pv = c.GetComponent<Il2CppPhoton.Pun.PhotonView>();
            if (pv == null) throw new Exception("no PhotonView on " + c.name + " for RPC " + method);
            pv.RPC(method, Il2CppPhoton.Pun.RpcTarget.All, new Il2CppInterop.Runtime.InteropTypes.Arrays.Il2CppReferenceArray<Il2CppSystem.Object>(args));
        }

        static Il2CppSystem.Object Box(int v)
        {
            var h = System.Runtime.InteropServices.GCHandle.Alloc(v, System.Runtime.InteropServices.GCHandleType.Pinned);
            try
            {
                var boxed = Il2CppInterop.Runtime.IL2CPP.il2cpp_value_box(Il2CppInterop.Runtime.Il2CppClassPointerStore<int>.NativeClassPtr, h.AddrOfPinnedObject());
                return new Il2CppSystem.Object(boxed);
            }
            finally { h.Free(); }
        }
        static Il2CppSystem.Object Box(int[] v) =>
            new Il2CppSystem.Object(new Il2CppInterop.Runtime.InteropTypes.Arrays.Il2CppStructArray<int>(v).Pointer);
        static Il2CppSystem.Object Box(bool[] v) =>
            new Il2CppSystem.Object(new Il2CppInterop.Runtime.InteropTypes.Arrays.Il2CppStructArray<bool>(v).Pointer);

        // Click the chosen cards in DCGO's open selection panel, then end it (or press "No Selection" to decline),
        // the same calls the panel's own buttons make. A click on the last allowed card may end the panel by itself.
        void DriveCardPanel(Il2Cpp.SelectCardPanel panel, List<Dictionary<string, string>> chosen, bool declined)
        {
            if (UnityEngine.Time.realtimeSinceStartup - _panelOpenSince < 1.0f)
                throw new Exception("card panel still opening");
            if (declined)
            {
                LoggerInstance.Msg("agent: card panel: No Selection");
                panel.OnClickNotSelectButton();
                return;
            }
            var shown = panel.GetComponentsInChildren<Il2Cpp.HandCard>(false);
            var used = new HashSet<int>();
            int picked = 0;
            // The panel's own capacity: clicking past it REPLACES the last pick (SelectCardPanel.OnClickHandCard), so
            // never click more cards than this panel takes - DCGO opens another panel for the rest.
            int maxCount = int.MaxValue;
            try { var mc = Member(panel, "_maxCount"); if (mc is int m && m > 0) maxCount = m; } catch { }
            var pre = new List<int>();
            try
            {
                var lst = Member(panel, "_preSelectedHandCardList") as Il2CppSystem.Collections.Generic.List<Il2Cpp.HandCard>;
                for (int j = 0; lst != null && j < lst.Count; j++) if (lst[j]?.cardSource != null) pre.Add(lst[j].cardSource.CardIndex);
            }
            catch { }
            // DCGO can split one engine prompt over several panels (e.g. "1 Digimon and 1 Tamer" = two panels): pick
            // only the chosen cards THIS panel shows; the rest are answered by the next request, once our replay has
            // applied this panel's answer.
            foreach (var pk in chosen)
            {
                Il2Cpp.HandCard hit = null;
                for (int i = 0; i < shown.Length; i++)
                {
                    var cs = shown[i]?.cardSource;
                    if (cs != null && cs.CardID == pk["card_id"] && used.Add(i)) { hit = shown[i]; break; }
                }
                if (hit == null) continue;
                if (picked >= maxCount) break;
                if (hit.cardSource != null && pre.Contains(hit.cardSource.CardIndex))
                { LoggerInstance.Msg("agent: card panel: " + pk["card_id"] + " already selected"); picked++; continue; }
                LoggerInstance.Msg("agent: card panel: pick " + pk["card_id"]);
                panel.OnClickHandCard(hit);
                picked++;
                if (!panel.gameObject.activeInHierarchy) return; // the last allowed pick ended the selection
            }
            if (picked == 0)
            {
                var ids = new List<string>();
                foreach (var h in shown) ids.Add(h?.cardSource?.CardID ?? "?");
                throw new Exception("card panel shows [" + string.Join(",", ids) + "], none of the picks");
            }
            LoggerInstance.Msg("agent: card panel: End Selection");
            panel.OnClickEndSelectButton();
        }

        // DCGO's command menu (SideBar "コマンド選択パネル"): UserSelectionManager int / bool menus and similar.
        // a hand prompt is open for the local seat when its hand cards carry a click target (a mandatory pick shows
        // no button at all: the player just clicks a card)
        static bool HandClickable(Il2Cpp.GManager gm)
        {
            var objs = gm?.You?.HandCardObjects;
            if (objs == null) return false;
            for (int i = 0; i < objs.Count; i++) if (objs[i] != null && objs[i].OnClickAction != null) return true;
            return false;
        }

        static UnityEngine.UI.Button FindActiveButtonPath(string pathPart)
        {
            foreach (var b in UnityEngine.Object.FindObjectsOfType<UnityEngine.UI.Button>())
                if (b != null && b.gameObject.activeInHierarchy && b.interactable && PathOf(b.transform).Contains(pathPart)) return b;
            return null;
        }

        static List<UnityEngine.UI.Button> MenuButtons()
        {
            var list = new List<UnityEngine.UI.Button>();
            foreach (var b in UnityEngine.Object.FindObjectsOfType<UnityEngine.UI.Button>())
                if (b != null && b.gameObject.activeInHierarchy && b.interactable && PathOf(b.transform).Contains("コマンド選択パネル"))
                    list.Add(b);
            return list;
        }

        static bool MenuOpen() => MenuButtons().Count > 0;

        static string ButtonText(UnityEngine.UI.Button b)
        {
            var t = b.GetComponentInChildren<UnityEngine.UI.Text>();
            if (t != null && !string.IsNullOrEmpty(t.text)) return t.text;
            var tmp = b.GetComponentInChildren<Il2CppTMPro.TMP_Text>();
            return tmp != null ? tmp.text : "";
        }

        // click the first menu button whose (lower-cased) text matches; the click runs DCGO's own handler (its RPC)
        bool ClickMenu(Func<string, bool> match)
        {
            var buttons = MenuButtons();
            var texts = buttons.Select(b => ButtonText(b).Trim().ToLowerInvariant()).ToList();
            for (int i = 0; i < buttons.Count; i++)
            {
                if (!match(texts[i])) continue;
                LoggerInstance.Msg("agent: menu: '" + texts[i] + "' of [" + string.Join(" | ", texts) + "]");
                buttons[i].onClick.Invoke();
                return true;
            }
            LoggerInstance.Warning("agent: menu: no match in [" + string.Join(" | ", texts) + "]");
            return false;
        }

        void AnswerHandPick(Il2Cpp.GManager gm, int you, List<Dictionary<string, string>> chosen)
        {
            // The local seat's hand prompt is UI: hand cards carry a click target and the side bar shows
            // "End Selection". Click like a human, so DCGO sends its own answer and tears its UI down; a queued
            // RPC answer alone left the UI (and the prompt) open.
            if (MenuOpen() || FindActiveButtonPath("戻るボタン") != null || HandClickable(gm))
            {
                // A "you may" effect first asks whether to activate it at all (OptionalSkill: an activate button plus
                // the back button) before the hand becomes selectable; the engine folds that into the pick (a pick
                // means yes). Confirm it, then the pick follows on the retry.
                if (!HandClickable(gm) && MenuOpen())
                {
                    if (ClickMenu(t => !(t == "end selection" || t.StartsWith("no selection") || t.StartsWith("don") || t.StartsWith("no"))))
                        throw new Exception("optional effect confirmed - the hand pick follows");
                    throw new Exception("hand not selectable and the menu offers only [" + string.Join(" | ", MenuButtons().Select(b => ButtonText(b).Trim())) + "]");
                }
                var objs = gm.You.HandCardObjects;
                var usedObj = new HashSet<int>();
                foreach (var pk in chosen)
                {
                    Il2Cpp.HandCard hit = null;
                    for (int i = 0; objs != null && i < objs.Count; i++)
                    {
                        var hc = objs[i];
                        if (hc?.cardSource?.CardID == pk["card_id"] && hc.OnClickAction != null && usedObj.Add(i)) { hit = hc; break; }
                    }
                    if (hit == null) throw new Exception("hand card " + pk["card_id"] + " is not clickable yet");
                    if (HandCardSelected(hit)) { LoggerInstance.Msg("agent: hand card " + pk["card_id"] + " already selected"); continue; }
                    LoggerInstance.Msg("agent: hand click " + pk["card_id"]);
                    hit.OnClickAction.Invoke(hit);
                }
                // DCGO shows "End Selection" a moment AFTER the click: until it is clicked the prompt is still open
                // (Glowing Dawn's ST23-03 sat selected with no End Selection), so retry until it is
                if (!MenuOpen()) throw new Exception("hand card clicked - waiting for End Selection");
                if (!ClickMenu(t => t == "end selection" || t.StartsWith("no selection")))
                    throw new Exception("hand prompt has no End Selection");
                return;
            }
            // a local (non-AI) seat answers through the UI only: an RPC sent before the UI is up is recorded, but the
            // UI stays open and DCGO waits on it
            if (!gm.IsAI) throw new Exception("hand prompt not open yet");
            var she = UnityEngine.Object.FindObjectOfType<Il2Cpp.SelectHandEffect>();
            var used = new HashSet<int>();
            var idx = new List<int>();
            var hand = gm.You.HandCards;
            foreach (var pk in chosen)
                for (int i = 0; i < hand.Count; i++)
                    if (hand[i]?.CardID == pk["card_id"] && used.Add(hand[i].CardIndex)) { idx.Add(hand[i].CardIndex); break; }
            if (idx.Count != chosen.Count) throw new Exception("picked hand card not found");
            LoggerInstance.Msg("agent: hand pick " + string.Join(",", chosen.ConvertAll(c => c["card_id"])));
            if (gm.IsAI) she.SetTargetHandCards(you, idx.ToArray());
            else Rpc(she, "SetTargetHandCards", Box(you), Box(idx.ToArray()));
        }

        // a selected hand card shows its pick number ("Parent/SelectedIndex"); clicking it again would deselect it
        static bool HandCardSelected(Il2Cpp.HandCard hc)
        {
            try
            {
                var t = hc.transform.Find("Parent/SelectedIndex");
                return t != null && t.gameObject.activeInHierarchy;
            }
            catch { return false; }
        }

        // A "you may" effect first asks whether to use it (OptionalSkill: a lone "Use" button + the back button); the
        // engine folds that into its pick (a pick = use, a decline = don't). Answer it; returns true when the decline
        // ended the prompt, throws (retry) after "Use" so the pick is answered once DCGO opens it.
        bool ConfirmOptional(bool declined, int useStamp = -1)
        {
            var texts = MenuButtons().Select(b => ButtonText(b).Trim().ToLowerInvariant()).ToList();
            if (texts.Count != 1 || texts[0] != "use") return false;
            var back = FindActiveButtonPath("戻るボタン");
            if (declined && back != null)
            {
                // this decline IS the engine's answer to its prompt (PASS): stamp it on DCGO's SetUseOptional row
                _engineAction = 62;
                LoggerInstance.Msg("agent: optional effect: not used (back button)");
                back.onClick.Invoke();
                return true;
            }
            // when the engine's pick IS the activation (the effect's own card, e.g. "suspend Close to ..."), "Use" is
            // that answer: stamp it on DCGO's SetUseOptional row
            _engineAction = useStamp;
            ClickMenu(t => t == "use");
            throw new Exception("optional effect confirmed - the pick follows");
        }

        static string PanelDescription(Il2Cpp.GManager gm)
        {
            try
            {
                var p = gm?.selectCardPanel;
                if (p == null || !p.gameObject.activeInHierarchy) return "";
                var cards = p.GetComponentsInChildren<Il2Cpp.HandCard>(false).Select(h => h?.cardSource?.CardID ?? "?");
                var texts = p.GetComponentsInChildren<UnityEngine.UI.Text>(false).Select(t => t?.text).Where(t => !string.IsNullOrWhiteSpace(t))
                    .Concat(p.GetComponentsInChildren<Il2CppTMPro.TMP_Text>(false).Select(t => t?.text).Where(t => !string.IsNullOrWhiteSpace(t)));
                return " panel=[" + string.Join(",", cards) + "] text=[" + string.Join(" | ", texts).Replace(Environment.NewLine, " ").Replace("\n", " ") + "]";
            }
            catch (Exception e) { return " panel=? " + e.Message; }
        }

        // DCGO asks Assembly one material element per panel; once the engine declined the assembly (its prompt has
        // moved on), every further element panel is declined too.
        bool DeclineStrayAssembly(Il2Cpp.GManager gm, string enginePrompt)
        {
            if (enginePrompt != null && enginePrompt.StartsWith("Assembly")) return false;
            var p = gm.selectCardPanel;
            var sce = gm.GetComponent<Il2Cpp.SelectCardEffect>();
            if (p == null || !p.gameObject.activeInHierarchy || sce == null || !sce._isAssembly) return false;
            // `_isAssembly` lingers on SelectCardEffect after an Assembly; MultipleSkills reuses the same card panel
            // for "Multiple effects are triggered", whose "No Selection" is "Don't activate these effects" (-1)
            if (PanelSays(p, "Multiple effects")) return false;
            if (UnityEngine.Time.realtimeSinceStartup - _panelOpenSince < 1.0f) return false;
            LoggerInstance.Warning("agent: DCGO asks another Assembly element the engine already declined - No Selection");
            p.OnClickNotSelectButton();
            return true;
        }

        static bool PanelSays(Il2Cpp.SelectCardPanel p, string text) =>
            p.GetComponentsInChildren<UnityEngine.UI.Text>(false).Any(t => t != null && t.text != null && t.text.Contains(text))
            || p.GetComponentsInChildren<Il2CppTMPro.TMP_Text>(false).Any(t => t != null && t.text != null && t.text.Contains(text));

        static bool FieldClickable()
        {
            foreach (var f in UnityEngine.Object.FindObjectsOfType<Il2Cpp.FieldPermanentCard>())
                if (f != null && f.gameObject.activeInHierarchy && f.OnClickAction != null) return true;
            return false;
        }

        static Il2Cpp.CardSource HandCard(Il2Cpp.Player p, string cardId, int slot)
        {
            var hand = p.HandCards;
            if (hand == null) return null;
            if (slot >= 0 && slot < hand.Count && hand[slot] != null && hand[slot].CardID == cardId) return hand[slot];
            for (int i = 0; i < hand.Count; i++) if (hand[i] != null && hand[i].CardID == cardId) return hand[i];
            return null;
        }

        static Il2Cpp.Permanent FieldByCard(Il2Cpp.Player p, string cardId, int compactIndex)
        {
            var bt = p.GetBattleAreaPermanents();
            if (bt == null) return null;
            if (compactIndex >= 0 && compactIndex < bt.Count && bt[compactIndex]?.TopCard?.CardID == cardId) return bt[compactIndex];
            for (int i = 0; i < bt.Count; i++) if (bt[i]?.TopCard?.CardID == cardId) return bt[i];
            return null;
        }

        void Perform(string text, Il2Cpp.GManager gm, Il2Cpp.TurnStateMachine tsm, float now)
        {
            var status = Field(text, "status");
            var kind = Field(text, "kind");
            LoggerInstance.Msg("agent: " + (status ?? "?") + " " + (kind ?? "") + " action " + (Field(text, "action_id") ?? "-") +
                               " p=" + (Field(text, "prob") ?? "-") + " v=" + (Field(text, "value") ?? "-") + (status == "act" ? "" : " " + (Field(text, "message") ?? Field(text, "decision_player") ?? "")));
            _agentRowsAtAct = RawLength();
            _agentRetryAt = now + 4f;
            {
                // The engine asks for a turn action, so it has no open choice; DCGO still asking the local seat whether
                // to use an optional effect ("Use" + back button) is one the engine resolved without a decision (no
                // legal target: a "may" with nothing to choose). Declining is the same outcome.
                // (also while the engine waits on the other seat: it then has no open choice for us either)
                var k0 = Field(text, "kind");
                if ((status == "wait" || k0 == "play" || k0 == "digivolve" || k0 == "attack" || k0 == "pass" || k0 == "activate")
                    && tsm.IsSelecting && TurnPlayerId() == gm.You.PlayerID)
                {
                    if (DeclineStrayAssembly(gm, null)) { _agentNextAt = now + 2f; return; }
                    // likewise a trigger-order panel ("Multiple effects are triggered"): the engine resolved those
                    // triggers without an ordering decision, so any order is the engine's; take the first
                    var tp0 = gm.selectCardPanel;
                    if (tp0 != null && tp0.gameObject.activeInHierarchy && PanelSays(tp0, "Multiple effects"))
                    {
                        var first = tp0.GetComponentsInChildren<Il2Cpp.HandCard>(false).FirstOrDefault(h => h?.cardSource != null);
                        if (first != null)
                        {
                            LoggerInstance.Warning("agent: DCGO asks a trigger order the engine did not - resolving " + first.cardSource.CardID + " first");
                            try { DriveCardPanel(tp0, new List<Dictionary<string, string>> { new Dictionary<string, string> { ["card_id"] = first.cardSource.CardID } }, false); }
                            catch (Exception e) { LoggerInstance.Warning("agent: " + e.Message); }
                            _agentNextAt = now + 2f;
                            return;
                        }
                    }
                    // a mandatory card panel whose candidates are all ONE card: DCGO listing one effect once per
                    // timing (BT24-040's "[On Play] [When Digivolving]" effect activated by BT24-102's end-of-turn
                    // effect is offered twice). The engine has one effect and no choice; take the first, marked stray.
                    if (tp0 != null && tp0.gameObject.activeInHierarchy && !PanelSays(tp0, "Multiple effects")
                        && FindActiveButton("NotSelectButton") == null)
                    {
                        var hc = tp0.GetComponentsInChildren<Il2Cpp.HandCard>(false).Where(h => h?.cardSource != null).ToList();
                        if (hc.Count >= 2 && hc.Select(h => h.cardSource.CardID).Distinct().Count() == 1
                            && UnityEngine.Time.realtimeSinceStartup - _panelOpenSince > 3f)
                        {
                            LoggerInstance.Warning("agent: DCGO asks to pick among " + hc.Count + " identical " + hc[0].cardSource.CardID +
                                                   " entries the engine has as one - taking the first");
                            _engineAction = -2;
                            try { DriveCardPanel(tp0, new List<Dictionary<string, string>> { new Dictionary<string, string> { ["card_id"] = hc[0].cardSource.CardID } }, false); }
                            catch (Exception e) { LoggerInstance.Warning("agent: " + e.Message); }
                            _agentNextAt = now + 2f;
                            return;
                        }
                    }
                    var texts = MenuButtons().Select(b => ButtonText(b).Trim().ToLowerInvariant()).ToList();
                    var back = FindActiveButtonPath("戻るボタン");
                    if (back != null && texts.Count == 1 && texts[0] == "use")
                    {
                        LoggerInstance.Warning("agent: DCGO asks to use an optional effect the engine resolved without a choice - declining");
                        _engineAction = -1; // no engine decision: the answer row carries no stamp and is not replayed
                        back.onClick.Invoke();
                        _agentNextAt = now + 2f;
                        return;
                    }
                }
            }
            if (status != "act") { _agentNextAt = now + 2f; return; }
            // the answer's identity: a corrected answer for the same step (e.g. a new agent build) is a new answer
            var sig = (Field(text, "action_id") ?? "") + "@" + (Field(text, "steps") ?? "") + "@" + (Field(text, "category") ?? "")
                      + "@" + string.Join(",", Picks(text).Select(pk => (pk["pick"] ?? "") + ":" + (pk["card_id"] ?? pk["index"] ?? "")));
            bool resend = false;
            if (sig == _agentLastSig)
            {
                // the same answer for the same step: DCGO has not logged the last action yet (an attack or effect still
                // resolving). Wait for it; only a long stall means DCGO really refused it.
                // Never perform the same answer twice while DCGO may still take it (a second queued attack or play would
                // be a real second action). Exception: a turn action DCGO dequeued and DROPPED (sent while it was still
                // settling) - after 30 s with DCGO idle, nothing queued and no prompt open, it provably did not happen,
                // so it is sent once more.
                var stallKind = Field(text, "kind");
                var openPanel = gm.selectCardPanel;
                var stallCat = Field(text, "category");
                if (stallKind == "selection" && now - _agentLastSigAt >= 12f && stallCat != "permanent" && stallCat != "hand"
                    && tsm.IsSelecting && TurnPlayerId() == gm.You.PlayerID && (MenuOpen() || HandClickable(gm)))
                {
                    LoggerInstance.Warning("agent: DCGO's prompt is still open 12 s after the " + stallCat + " answer - driving it again");
                    resend = true;
                }
                else if (stallKind == "selection" && now - _agentLastSigAt >= 12f && Field(text, "category") == "hand" && MenuOpen())
                {
                    // the hand pick was clicked but its End Selection button appeared only afterwards
                    LoggerInstance.Warning("agent: hand prompt still open 12 s after the answer - driving it again");
                    resend = true;
                }
                else if (stallKind == "selection" && now - _agentLastSigAt >= 12f && openPanel != null && openPanel.gameObject.activeInHierarchy)
                {
                    LoggerInstance.Warning("agent: card panel still open 12 s after the answer - driving it again (selected "
                        + (openPanel.SelectedList?.Count ?? 0) + ")");
                    resend = true;
                }
                else if (now - _agentLastSigAt >= 30f && !_agentResent
                    && (stallKind == "play" || stallKind == "digivolve" || stallKind == "attack")
                    && !tsm.IsSelecting && DcgoReadyForTurnAction(tsm, gm.You))
                {
                    LoggerInstance.Warning("agent: DCGO idle 30 s after '" + stallKind + "' with nothing queued - it was dropped; sending it again");
                    _agentResent = true;
                    resend = true;
                }
                else if (now - _agentLastSigAt >= 60f && !_agentStallWarned)
                {
                    LoggerInstance.Warning("agent: DCGO has not logged the last action for 60 s (same answer, no progress) - still waiting, not repeating it");
                    try
                    {
                        // what DCGO is waiting on, for the log (the screen is not visible to the harness)
                        var buttons = UnityEngine.Object.FindObjectsOfType<UnityEngine.UI.Button>()
                            .Where(b => b != null && b.gameObject.activeInHierarchy && b.interactable).Select(b => b.name);
                        LoggerInstance.Warning("agent: stall state: phase=" + tsm.gameContext.TurnPhase + " turnPlayer=" + TurnPlayerId() +
                            " isSelecting=" + tsm.IsSelecting + " attacking=" + (tsm.AttackingPermanent?.TopCard?.CardID ?? "-") +
                            " defending=" + (tsm.DefendingPermanent?.TopCard?.CardID ?? "-") + " buttons=[" + string.Join(",", buttons) + "]"
                            + PanelDescription(gm));
                    }
                    catch (Exception e) { LoggerInstance.Warning("agent: stall state unavailable: " + e.Message); }
                    _agentStallWarned = true;
                }
                if (!resend)
                {
                    _agentNextAt = now + 2f;
                    return;
                }
            }
            else _agentResent = false;
            _agentLastSig = sig;
            _agentLastSigAt = now;
            _agentStallWarned = false;
            var me = gm.You;
            var opp = gm.Opponent;
            // a turn action is only queued while DCGO's main-phase loop is idle (see DcgoReadyForTurnAction); an answer
            // to an open prompt is never held back (the prompt is what DCGO is waiting on)
            if ((kind == "play" || kind == "digivolve" || kind == "attack" || kind == "pass" || kind == "activate") && !DcgoReadyForTurnAction(tsm, gm.You))
                throw new Exception("DCGO is still resolving the previous action - " + kind + " held" + PanelDescription(gm));
            switch (kind)
            {
                case "mulligan_keep": ClickUiButtonNamed("NotSelectButton"); break;
                case "mulligan_redraw": ClickUiButtonNamed("EndSelectButton"); break;
                case "pass": ClickUiButtonNamed("Btn_Skill00"); break;
                case "hatch":
                case "move_from_breeding":
                    // SendShouldHatch answers for the turn player: never send it outside our own breeding phase
                    if (TurnPlayerId() != me.PlayerID || tsm.TurnCount < 1) throw new Exception("not our breeding phase yet");
                    tsm.SendShouldHatch(true);
                    break;
                case "play":
                case "digivolve":
                    {
                        var card = HandCard(me, Field(text, "card_id"), int.Parse(Field(text, "hand_slot") ?? "-1"));
                        if (card == null) throw new Exception("hand card " + Field(text, "card_id") + " not found");
                        var frames = me.fieldCardFrames;
                        int frame = -1;
                        if (kind == "digivolve")
                        {
                            if (Field(text, "target_breeding") == "true") frame = frames.Count - 1; // the breeding frame is the last one
                            else
                            {
                                var target = FieldByCard(me, Field(text, "target_card_id"), int.Parse(Field(text, "target_field") ?? "-1"));
                                if (target == null) throw new Exception("digivolve target not found");
                                frame = frames.IndexOf(target.PermanentFrame);
                            }
                        }
                        else
                        {
                            // a new permanent goes to the first empty battle frame (the last frame is the breeding area)
                            for (int i = 0; i < frames.Count - 1; i++) if (frames[i].IsEmptyFrame()) { frame = i; break; }
                        }
                        LoggerInstance.Msg("agent: " + kind + " " + card.CardID + " (index " + card.CardIndex + ") -> frame " + frame);
                        tsm.QueueMainPhaseAction(me, new Il2Cpp.PlayCardAction(card.CardIndex, frame, new int[0], -1, new int[0]));
                        break;
                    }
                case "attack":
                    {
                        // AttackPermanentAction carries indices into GetFieldPermanents(), resolved when DCGO DEQUEUES the
                        // action. Resolve them by card id + occurrence in that same list, and only once the board has
                        // held still (a breeding move or deletion still settling reorders it).
                        var boardSig = FieldSig(me) + "|" + FieldSig(opp);
                        if (boardSig != _boardSig || now - _boardSigAt < 1f)
                        {
                            if (boardSig != _boardSig) { _boardSig = boardSig; _boardSigAt = now; }
                            throw new Exception("board settling - attack held");
                        }
                        var attackerId = Field(text, "attacker_card_id");
                        int a = FieldIndexByCard(me, attackerId, int.Parse(Field(text, "attacker_occurrence") ?? "0"));
                        if (a < 0) throw new Exception("attacker " + attackerId + " not found");
                        int d = -1;
                        if (Field(text, "target_security") != "true")
                        {
                            d = FieldIndexByCard(opp, Field(text, "target_card_id"), int.Parse(Field(text, "target_occurrence") ?? "0"));
                            if (d < 0) throw new Exception("attack target not found");
                        }
                        LoggerInstance.Msg("agent: attack " + attackerId + " (" + a + ") -> " + d);
                        tsm.QueueMainPhaseAction(me, new Il2Cpp.AttackPermanentAction(a, d));
                        break;
                    }
                case "activate":
                    {
                        // ActivatePermanentAction indices are resolved at dequeue time: card id + occurrence in
                        // GetFieldPermanents(), once the board has held still (as for attacks)
                        var boardSig2 = FieldSig(me) + "|" + FieldSig(opp);
                        if (boardSig2 != _boardSig || now - _boardSigAt < 1f)
                        {
                            if (boardSig2 != _boardSig) { _boardSig = boardSig2; _boardSigAt = now; }
                            throw new Exception("board settling - activate held");
                        }
                        var actId = Field(text, "card_id");
                        int pi = FieldIndexByCard(me, actId, int.Parse(Field(text, "occurrence") ?? "0"));
                        if (pi < 0) throw new Exception("permanent " + actId + " to activate not found");
                        // the engine's sub-slot is SEMANTIC (2 = [Main], 3 = <Link>); DCGO's SkillIndex is POSITIONAL,
                        // into the permanent's EffectList(OnDeclaration): the first activatable effect that is not the
                        // link declaration, or the link declaration (DCGO's Harness InputDriver.FindDeclarableSkillIndex).
                        // Sending the sub-slot itself activated nothing for P-206's <Delay> (its only declarable effect).
                        int sub = int.Parse(Field(text, "skill_index") ?? "2");
                        bool wantLink = sub == 3;
                        int skill = -1;
                        var effs = me.GetFieldPermanents()[pi].EffectList(Il2Cpp.EffectTiming.OnDeclaration);
                        for (int i = 0; effs != null && i < effs.Count; i++)
                        {
                            var e = effs[i];
                            if (e == null || e.TryCast<Il2Cpp.ActivateClass>() == null) continue;
                            bool isLink = e.EffectName != null && e.EffectName.StartsWith("Link (Cost:");
                            if (isLink == wantLink) { skill = i; break; }
                        }
                        if (skill < 0) throw new Exception("no declarable " + (wantLink ? "link" : "[Main]") + " effect on " + actId);
                        LoggerInstance.Msg("agent: activate " + actId + " (" + pi + ") skill " + skill + " (engine sub-slot " + sub + ")");
                        tsm.QueueMainPhaseAction(me, new Il2Cpp.ActivatePermanentAction(pi, skill));
                        break;
                    }
                case "selection":
                    AnswerSelection(text, gm, tsm, now);
                    break;
                default:
                    LoggerInstance.Warning("agent: '" + kind + "' is not performed yet - leaving this decision to the human");
                    _agentRetryAt = now + 30f;
                    break;
            }
        }

        string _boardSig;
        float _boardSigAt;

        static string FieldSig(Il2Cpp.Player p) => PermIds(p?.GetFieldPermanents());

        // index into ONE GetFieldPermanents() list of the occurrence-th permanent whose top card is cardId
        static int FieldIndexByCard(Il2Cpp.Player p, string cardId, int occurrence)
        {
            var fp = p?.GetFieldPermanents();
            if (fp == null) return -1;
            int seen = 0;
            for (int i = 0; i < fp.Count; i++)
            {
                if (fp[i]?.TopCard?.CardID != cardId) continue;
                if (seen == occurrence) return i;
                seen++;
            }
            return -1;
        }

        static string PermIds(Il2CppSystem.Collections.Generic.List<Il2Cpp.Permanent> list)
        {
            var sb = new StringBuilder("[");
            if (list != null)
                for (int i = 0; i < list.Count; i++)
                {
                    if (i > 0) sb.Append(',');
                    var top = list[i]?.TopCard;
                    sb.Append(top == null ? "null" : Quote(top.CardID));
                }
            return sb.Append(']').ToString();
        }

        static string Ids(Il2CppSystem.Collections.Generic.List<Il2Cpp.CardSource> list)
        {
            var sb = new StringBuilder("[");
            if (list != null)
                for (int i = 0; i < list.Count; i++)
                {
                    if (i > 0) sb.Append(',');
                    var c = list[i];
                    sb.Append(c == null ? "null" : Quote(c.CardID));
                }
            return sb.Append(']').ToString();
        }

        // The zones of both seats as card-id lists, index 0 first (library: top of the deck first, as far as the
        // game orders it). Written at Init and at the first mulligan so the converter can rebuild the post-shuffle
        // deck orders (hand + security + library) and the digitama (egg) decks for the game_start row.
        static string seatsQueued(Il2Cpp.GManager gm)
        {
            var a = gm.GetPlayerFromID(0); var b = gm.GetPlayerFromID(1);
            return (a != null && a.HasMainPhaseAction() ? "true" : "false") + "," + (b != null && b.HasMainPhaseAction() ? "true" : "false");
        }

        static void WriteSnapshot(string tag)
        {
            try
            {
                var gm = Il2Cpp.GManager.instance;
                if (gm == null || gm.You == null || gm.Opponent == null) return;
                _snapshots++;
                var sb = new StringBuilder();
                sb.Append("{\"seq\":").Append(System.Threading.Interlocked.Increment(ref _seq))
                  .Append(",\"t\":\"").Append(DateTime.UtcNow.ToString("o"))
                  .Append("\",\"method\":\"Snapshot.").Append(tag).Append("\",\"is_ai\":").Append(gm.IsAI ? "true" : "false")
                  .Append(",\"you\":").Append(gm.You.PlayerID).Append(",\"turn_count\":").Append(gm.turnStateMachine != null ? gm.turnStateMachine.TurnCount : -1);
                try
                {
                    var tsm = gm.turnStateMachine;
                    sb.Append(",\"dcgo\":{\"phase\":").Append(Quote(tsm.gameContext.TurnPhase.ToString()))
                      .Append(",\"turn_player\":").Append(TurnPlayerId())
                      .Append(",\"selecting\":").Append(tsm.IsSelecting ? "true" : "false")
                      .Append(",\"attacking\":").Append(Quote((Member(tsm, "AttackingPermanent") as Il2Cpp.Permanent)?.TopCard?.CardID ?? ""))
                      .Append(",\"queued\":[").Append(seatsQueued(gm)).Append("]}");
                }
                catch { }
                sb.Append(",\"players\":[");
                var seats = new[] { gm.GetPlayerFromID(0), gm.GetPlayerFromID(1) };
                for (int i = 0; i < 2; i++)
                {
                    var p = seats[i];
                    if (i > 0) sb.Append(',');
                    sb.Append("{\"id\":").Append(p.PlayerID)
                      .Append(",\"library\":").Append(Ids(p.LibraryCards))
                      .Append(",\"hand\":").Append(Ids(p.HandCards))
                      .Append(",\"security\":").Append(Ids(p.SecurityCards))
                      .Append(",\"egg_library\":").Append(Ids(p.DigitamaLibraryCards))
                      .Append(",\"trash\":").Append(Ids(p.TrashCards))
                      .Append(",\"breeding\":").Append(PermIds(p.GetBreedingAreaPermanents()))
                      .Append(",\"battle\":").Append(PermIds(p.GetBattleAreaPermanents()));
                    try { sb.Append(",\"memory\":").Append(p.MemoryForPlayer); } catch { }
                    sb.Append('}');
                }
                sb.Append("]}");
                lock (Gate)
                    File.AppendAllText(Path.Combine(_dir, "raw-" + _session + ".jsonl"), sb.ToString() + "\n");
            }
            catch (Exception e) { MelonLogger.Warning("snapshot failed: " + e.Message); }
        }

        static int _seq;

        // Harmony injects the patched method and its boxed arguments by these special parameter names.
        static void ProbePostfix(MethodBase __originalMethod, object[] __args)
        {
            try
            {
                var key = (__originalMethod.DeclaringType?.Name ?? "?") + "." + __originalMethod.Name;
                if (key == "TurnStateMachine.EndGame") _gameEnded = true;
                var sb = new StringBuilder();
                sb.Append("{\"seq\":").Append(System.Threading.Interlocked.Increment(ref _seq))
                  .Append(",\"t\":\"").Append(DateTime.UtcNow.ToString("o"))
                  .Append("\",\"method\":\"").Append(key).Append("\",\"args\":[");
                if (__args != null)
                    for (int i = 0; i < __args.Length; i++)
                    {
                        if (i > 0) sb.Append(',');
                        sb.Append(Json(__args[i]));
                    }
                sb.Append("],\"tp\":").Append(TurnPlayerId()).Append(",\"turn\":").Append(CurrentTurn());
                sb.Append(SelectionExtras(null, key, __args)).Append('}');
                lock (Gate)
                {
                    File.AppendAllText(Path.Combine(_dir, "raw-" + _session + ".jsonl"), sb.ToString() + "\n");
                }
                // The pre-mulligan snapshots (one per seat's SetRedraw) are counted PER GAME: Init starts a new game.
                // A session-wide count (the old `_snapshots < 2`) left every game after a launch's first without
                // its pre-mulligan zones.
                if (__originalMethod.Name == "Init") _redrawSnapshots = 0;
                bool redrawSnap = __originalMethod.Name == "SetRedraw" && _redrawSnapshots < 2;
                if (redrawSnap) _redrawSnapshots++;
                if (__originalMethod.Name == "Init" || redrawSnap
                    || __originalMethod.Name == "PassTurn" || __originalMethod.Name == "EndGame")
                    WriteSnapshot(__originalMethod.Name);
            }
            catch { /* never let logging disturb the game */ }
        }

        static readonly string _session = DateTime.UtcNow.ToString("yyyyMMddTHHmmssZ");

        // Minimal JSON for the argument kinds these methods take: numbers, bools, strings, arrays, and
        // otherwise the type name plus ToString (game objects; the converter reads them via later hooks).
        static string Json(object v)
        {
            switch (v)
            {
                case null: return "null";
                case bool b: return b ? "true" : "false";
                case int _: case long _: case short _: case byte _: case sbyte _: case uint _: case ulong _: case ushort _:
                    return Convert.ToString(v, System.Globalization.CultureInfo.InvariantCulture);
                case float f: return f.ToString("R", System.Globalization.CultureInfo.InvariantCulture);
                case double d: return d.ToString("R", System.Globalization.CultureInfo.InvariantCulture);
                case string str: return Quote(str);
                case System.Collections.IEnumerable seq:
                    {
                        var parts = new List<string>();
                        foreach (var item in seq) { parts.Add(Json(item)); if (parts.Count >= 256) break; }
                        return "[" + string.Join(",", parts) + "]";
                    }
                case Il2Cpp.CardSource cs: return CardJson(cs);
                case Il2Cpp.Permanent pm: return "{\"permanent_top\":" + CardJson(pm.TopCard) + "}";
                case Il2Cpp.ICardEffect ce: return "{\"effect\":" + Quote(ce.EffectName) + ",\"source\":" + CardJson(ce.EffectSourceCard) + "}";
                default: return Quote(v.GetType().Name + ":" + v);
            }
        }

        static string Quote(string s)
        {
            var sb = new StringBuilder("\"");
            foreach (var c in s)
            {
                if (c == '"') sb.Append("\\\"");
                else if (c == '\\') sb.Append("\\\\");
                else if (c < 0x20) sb.Append("\\u").Append(((int)c).ToString("x4"));
                else sb.Append(c);
            }
            return sb.Append('"').ToString();
        }
    }
}
