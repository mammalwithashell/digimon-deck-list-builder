using System;
using System.Text.Json;
using System.Collections.Generic;
using System.Linq;
using Digimon.Core.Constants;

using Digimon.Core.Loggers;

namespace Digimon.Core
{
    public class Game
    {
        public Player Player1 { get; private set; }
        public Player Player2 { get; private set; }
        public Player CurrentPlayer { get; set; }
        public Player OpponentPlayer => CurrentPlayer == Player1 ? Player2 : Player1;
        public TurnStateMachine TurnStateMachine { get; private set; }
        public bool IsGameOver { get; private set; }
        public Player? Winner { get; private set; }
        
        public IGameLogger Logger { get; private set; }

        private static readonly JsonSerializerOptions _jsonOptions = new() { WriteIndented = true };

        // Shared Memory Gauge: Positive for P1, Negative for P2.
        // P1 turn: 1 to 10 (Max), P2 side is < 0.
        // P2 turn: -1 to -10 (Max), P1 side is > 0.
        public int MemoryGauge { get; private set; }

        public Game(IGameLogger? logger = null)
        {
            Logger = logger ?? new SilentLogger();
            Player1 = new Player(1, "Player 1");
            Player2 = new Player(2, "Player 2");
            CurrentPlayer = Player1;
            TurnStateMachine = new TurnStateMachine(this);
        }

        public void StartGame(System.Collections.Generic.List<Card> deck1, System.Collections.Generic.List<Card> deck2)
        {
            Player1.SetupDeck(deck1);
            Player2.SetupDeck(deck2);

            Player1.SetupSecurity(5);
            Player2.SetupSecurity(5);

            // Set Initial Memory
            MemoryGauge = 0;

            // Initial Draw
            for(int i=0; i<5; i++)
            {
                Player1.Draw(this.Logger);
                Player2.Draw(this.Logger);
            }

            // Decide who goes first (random or fixed)
            CurrentPlayer = Player1;
            Player1.IsMyTurn = true;
            Player2.IsMyTurn = false;

            // Start First Turn
            TurnStateMachine.StartTurn();
        }

        public int GetMemory(Player player)
        {
            if (player == Player1) return MemoryGauge;
            return -MemoryGauge;
        }

        public void AddMemory(Player player, int amount)
        {
            if (player == Player1)
            {
                MemoryGauge += amount;
                if (MemoryGauge > 10) MemoryGauge = 10;
            }
            else
            {
                MemoryGauge -= amount;
                if (MemoryGauge < -10) MemoryGauge = -10;
            }
        }

        public void PayCost(Player player, int cost)
        {
            // Paying cost means moving towards opponent's side (subtracting memory from self)
            // If P1 pays, MemoryGauge decreases.
            // If P2 pays, MemoryGauge increases (from negative to positive).
            if (player == Player1)
            {
                MemoryGauge -= cost;
                if (MemoryGauge < -10) MemoryGauge = -10;
            }
            else
            {
                MemoryGauge += cost;
                if (MemoryGauge > 10) MemoryGauge = 10;
            }
        }

        public float[] GetBoardStateTensor(int playerId)
        {
            // Tensor Layout (Total Size: 680 floats):
            // ---------------------------------------------------------
            // [0-9] GLOBAL DATA (10 floats)
            // 0: Turn Count
            // 1: Current Phase (Enum Int)
            // 2: Memory Gauge (Relative to requesting player)
            // 3-9: Reserved / Padding (Index 8: PendingAction)
            //
            // [10-249] MY BATTLE AREA (12 Slots * 20 Floats = 240 floats)
            // Each Slot Structure:
            //   Offset + 0: Top Card Internal ID (from CardRegistry)
            //   Offset + 1: Current DP
            //   Offset + 2: Is Suspended (1.0 = Yes, 0.0 = No)
            //   Offset + 3: Has Used OPT (1.0 = Yes, 0.0 = No)
            //   Offset + 4: Source Count
            //   Offset + 5-19: Source Card IDs (Bottom to Top, max 15)
            // Slots 0-11 represent the field positions.
            //
            // [250-489] OPPONENT BATTLE AREA (12 Slots * 20 Floats = 240 floats)
            // Same structure as My Battle Area.
            //
            // [490-509] MY HAND (20 floats)
            // List of Card IDs. Paddings are 0.
            //
            // [510-529] OPPONENT HAND (20 floats)
            // List of Card IDs (Perfect Information assumption for training).
            //
            // [530-574] MY TRASH (45 floats)
            // Top 45 Card IDs in Trash.
            //
            // [575-619] OPPONENT TRASH (45 floats)
            // Top 45 Card IDs in Trash.
            //
            // [620-629] MY SECURITY STACK (10 floats)
            // Top 10 Card IDs (Revealed/Known state assumption or placeholders).
            //
            // [630-639] OPPONENT SECURITY STACK (10 floats)
            // Top 10 Card IDs.
            //
            // [640-659] MY BREEDING AREA (1 Slot * 20 Floats = 20 floats)
            // Standard slot structure.
            //
            // [660-679] OPPONENT BREEDING AREA (1 Slot * 20 Floats = 20 floats)
            // Standard slot structure.
            // ---------------------------------------------------------

            List<float> tensor = [];

            Player me = (playerId == 1) ? Player1 : Player2;
            Player opp = (playerId == 1) ? Player2 : Player1;

            // --- Global Data [0-9] ---
            tensor.Add(TurnStateMachine.TurnCount);
            tensor.Add((float)TurnStateMachine.CurrentPhase);
            // Memory relative to me
            tensor.Add(GetMemory(me));
            // Pad remaining 7
            for(int i=0; i<7; i++) tensor.Add(0);

            // --- My Field [12 slots * 20] ---
            AppendFieldData(tensor, me, 12);

            // --- Opp Field [12 slots * 20] ---
            AppendFieldData(tensor, opp, 12);

            // --- My Hand [20] ---
            AppendListIds(tensor, me.Hand, 20);

            // --- Opp Hand [20] ---
            AppendListIds(tensor, opp.Hand, 20);

            // --- My Trash [45] ---
            AppendListIds(tensor, me.Trash, 45);

            // --- Opp Trash [45] ---
            AppendListIds(tensor, opp.Trash, 45);

            // --- My Security [10] ---
            AppendListIds(tensor, me.Security, 10);

            // --- Opp Security [10] ---
            AppendListIds(tensor, opp.Security, 10);

            // --- My Breeding [1 slot * 20] ---
            AppendFieldData(tensor, me, 1, true);

            // --- Opp Breeding [1 slot * 20] ---
            AppendFieldData(tensor, opp, 1, true);

            return [.. tensor];
        }

        private static void AppendFieldData(List<float> tensor, Player p, int slots, bool isBreeding = false)
        {
            var sourceList = isBreeding ? p.BreedingArea : p.BattleArea;

            for (int i = 0; i < slots; i++)
            {
                if (i < sourceList.Count)
                {
                    Permanent perm = sourceList[i];
                    Card c = perm.TopCard;
                    // 1. Internal ID
                    tensor.Add(CardRegistry.GetId(c.Id));
                    // 2. DP
                    tensor.Add(perm.CurrentDP);
                    // 3. Suspended
                    tensor.Add(perm.IsSuspended ? 1.0f : 0.0f);
                    // 4. Has Used OPT
                    tensor.Add(perm.HasUsedOpt ? 1.0f : 0.0f);
                    // 5. Source Count
                    tensor.Add(perm.Sources.Count);
                    // 6-20. Sources (15 slots)
                    for(int j=0; j<15; j++) 
                    {
                        if (j < perm.Sources.Count)
                            tensor.Add(CardRegistry.GetId(perm.Sources[j].Id));
                        else
                            tensor.Add(0);
                    }
                }
                else
                {
                    // Empty Slot (20 floats)
                    for(int j=0; j<20; j++) tensor.Add(0);
                }
            }
        }

        private static void AppendListIds(List<float> tensor, List<Card> cards, int limit)
        {
            for (int i = 0; i < limit; i++)
            {
                if (i < cards.Count)
                    tensor.Add(CardRegistry.GetId(cards[i].Id));
                else
                    tensor.Add(0);
            }
        }

        public void EndGame(Player winner)
        {
            IsGameOver = true;
            Winner = winner;
        }

        public float[] GetActionMask(int playerId)
        {
             // Returns a boolean mask as floats (1.0 = valid, 0.0 = invalid)
             // Size: 2120 (Covering all ranges)
             float[] mask = new float[2120]; 
             
             // Default all to 0
             Array.Fill(mask, 0.0f);

             Player me = (playerId == 1) ? Player1 : Player2;
             GamePhase phase = TurnStateMachine.CurrentPhase;

             if (phase == GamePhase.Main)
             {
                 // 1. Play Cards (0-29)
                 for(int i=0; i < me.Hand.Count; i++)
                 {
                     mask[i] = 1.0f;
                 }

                 // 2. Trash (30-59) - Check effects? kept 0 for now.
                 
                 // 3. Attack (100-399)
                 for(int i=0; i<me.BattleArea.Count; i++)
                 {
                     Permanent attacker = me.BattleArea[i];
                     if (attacker.IsSuspended) continue; 
                     
                     // Security Attack (Target 12)
                     mask[100 + (i * 15) + 12] = 1.0f;

                     // Digimon Attacks (Targets 0-11)
                     for(int j=0; j<OpponentPlayer.BattleArea.Count; j++)
                     {
                         Permanent target = OpponentPlayer.BattleArea[j];
                         if (target.IsSuspended) 
                         {
                             mask[100 + (i * 15) + j] = 1.0f;
                         }
                     }
                 }

                 // 4. Digivolve (400-999)
                 for(int h=0; h<me.Hand.Count; h++)
                 {
                     Card evolveCard = me.Hand[h];
                     if (!evolveCard.IsDigimon) continue;

                     for(int f=0; f<me.BattleArea.Count; f++)
                     {
                         Permanent baseMon = me.BattleArea[f];
                         bool colorMatch = baseMon.TopCard.Colors.Intersect(evolveCard.Colors).Any();
                         bool levelValid = (evolveCard.Level - baseMon.TopCard.Level) == 1;

                         if (colorMatch && levelValid)
                         {
                             mask[400 + (h * 15) + f] = 1.0f;
                         }
                     }
                 }
                 
                 // 5. Effects (1000-1999)
                 
                 // 6. Pass (62) - End Turn
                 mask[62] = 1.0f;
             }
             else if (phase == GamePhase.Breeding)
             {
                 // Hatch (60)
                 if (me.BreedingArea.Count == 0 && me.DigitamaDeck.Count > 0) mask[60] = 1.0f;
                 // Move (61)
                 if (me.BreedingArea.Count > 0 && me.BreedingArea[0].TopCard.Level >= 3) mask[61] = 1.0f;
                 // Pass (62)
                 mask[62] = 1.0f;
             }
             else if (phase == GamePhase.SelectTarget) 
             {
                 // Enable logical candidates (placeholder)
             }
             else if (phase == GamePhase.SelectTrash)
             {
                  for(int i=0; i<me.Trash.Count; i++) mask[i] = 1.0f;
             }
             else if (phase == GamePhase.BlockTiming)
             {
                 mask[62] = 1.0f; // Pass
                 for(int i=0; i<me.BattleArea.Count; i++)
                 {
                     Permanent p = me.BattleArea[i];
                     if (!p.IsSuspended && p.HasKeyword("Blocker")) 
                     {
                         mask[100 + i] = 1.0f;
                     }
                 }
             }
             else if (phase == GamePhase.CounterTiming)
             {
                  mask[62] = 1.0f; // Pass
             }

             return mask;
        }

        public void SwitchTurn()
        {
            CurrentPlayer.IsMyTurn = false;
            CurrentPlayer = OpponentPlayer;
            CurrentPlayer.IsMyTurn = true;

            TurnStateMachine.StartTurn();
        }

        // --- Breeding Phase Actions (Agent Controlled) ---

        public void BreedingHatch()
        {
            if (TurnStateMachine.CurrentPhase != GamePhase.Breeding) return;
            
            // Logic handled by Player, but we wrap it to advance phase
            int initialCount = CurrentPlayer.BreedingArea.Count;
            CurrentPlayer.Hatch(this.Logger);
            
            // Validate if hatch actually happened (it might fail if area full or empty deck)
            // But for now we assume if it didn't error, proceed.
            // If the count changed, it was successful.
            if (CurrentPlayer.BreedingArea.Count > initialCount)
            {
                TurnStateMachine.OnBreedingActionCompleted();
            }
        }

        public void BreedingMove()
        {
            if (TurnStateMachine.CurrentPhase != GamePhase.Breeding) return;

            if (CurrentPlayer.BreedingArea.Count == 0) return;
            if (CurrentPlayer.BreedingArea[0].TopCard.Level < 3) return;

            int initialCount = CurrentPlayer.BreedingArea.Count;
            CurrentPlayer.MoveBreedingToBattle(this.Logger);

            // Check if move happened
            if (CurrentPlayer.BreedingArea.Count < initialCount)
            {
                TurnStateMachine.OnBreedingActionCompleted();
            }
        }

        public void BreedingPass()
        {
            if (TurnStateMachine.CurrentPhase != GamePhase.Breeding) return;
            TurnStateMachine.SkipBreedingPhase();
        }

        public string ToJson()
        {
            var state = new
            {
                TurnStateMachine.TurnCount,
                CurrentPhase = TurnStateMachine.CurrentPhase.ToString(),
                CurrentPlayer = CurrentPlayer.Id,
                MemoryGauge,
                IsGameOver,
                Winner = Winner?.Id,
                Player1 = new
                {
                    Player1.Id,
                    Memory = GetMemory(Player1),
                    HandCount = Player1.Hand.Count,
                    HandIds = Player1.Hand.Select(c => c.Id).ToList(),
                    SecurityCount = Player1.Security.Count,
                    DeckCount = Player1.Deck.Count,
                    BattleAreaCount = Player1.BattleArea.Count
                },
                Player2 = new
                {
                    Player2.Id,
                    Memory = GetMemory(Player2),
                    HandCount = Player2.Hand.Count,
                    HandIds = Player2.Hand.Select(c => c.Id).ToList(),
                    SecurityCount = Player2.Security.Count,
                    DeckCount = Player2.Deck.Count,
                    BattleAreaCount = Player2.BattleArea.Count
                }
            };

            return JsonSerializer.Serialize(state, _jsonOptions);
        }
        public void ExecuteAttack(int attackerIndex, int targetIndex)
        {
             // 1. Validate Phase
             if (TurnStateMachine.CurrentPhase != GamePhase.Main) return;

             // 2. Get Attacker
             if (attackerIndex < 0 || attackerIndex >= CurrentPlayer.BattleArea.Count) return;
             Permanent attacker = CurrentPlayer.BattleArea[attackerIndex];

             if (attacker.IsSuspended) 
             {
                 Logger.LogVerbose($"[Attack] Invalid: {attacker.TopCard.Name} is already suspended.");
                 return;
             }
             
             // 3. Suspend Attacker (Attack Cost)
             attacker.Suspend();
             Logger.Log($"[Attack] {CurrentPlayer.Name}'s {attacker.TopCard.Name} (DP: {attacker.CurrentDP}) Attacks!");

             // 4. Determine Target
             // Target 12 = Security
             // Target 0-11 = Digimon
             if (targetIndex == 12)
             {
                 Logger.LogVerbose($"[Attack] Target: Security Stack ({OpponentPlayer.Security.Count} cards)");
                 ResolveSecurityBattle(attacker);
             }
             else
             {
                 if (targetIndex < 0 || targetIndex >= OpponentPlayer.BattleArea.Count) return;
                 Permanent defender = OpponentPlayer.BattleArea[targetIndex];

                 if (!defender.IsSuspended)
                 {
                      Logger.LogVerbose($"[Attack] Target: {defender.TopCard.Name} (Unsuspend) - Usually blocks, but direct attack invalid.");
                      // Rules: usually can only attack Suspended Digimon. 
                      // Check for "Blocker" or "Redirect" later. 
                      // For now, allow direct attack logic only if suspended, or assume ActionMask filtered this.
                 }

                 Logger.LogVerbose($"[Attack] Target: {defender.TopCard.Name} (DP: {defender.CurrentDP})");
                 ResolveBattle(attacker, defender);
             }
             
             // 5. Post-Attack Check
             TurnStateMachine.CheckTurnEnd();
        }

        private void ResolveBattle(Permanent attacker, Permanent defender)
        {
            // Determine Metric: DP or Iceclad (Sources)
            bool useIceclad = attacker.HasKeyword("Iceclad") || defender.HasKeyword("Iceclad");
            
            int attackerValue = useIceclad ? attacker.Sources.Count : attacker.CurrentDP;
            int defenderValue = useIceclad ? defender.Sources.Count : defender.CurrentDP;

            string metricName = useIceclad ? "Sources" : "DP";
            Logger.LogVerbose($"[Battle] {attacker.TopCard.Name} ({attackerValue} {metricName}) vs {defender.TopCard.Name} ({defenderValue} {metricName})");

            if (attackerValue > defenderValue)
            {
                Logger.Log($"[Battle] Attacker Wins! Deleting Defender.");
                DeletePermanent(defender);
            }
            else if (attackerValue < defenderValue)
            {
                Logger.Log($"[Battle] Defender Wins! Deleting Attacker.");
                DeletePermanent(attacker);
            }
            else
            {
                Logger.Log($"[Battle] Tie! Both Deleted.");
                DeletePermanent(attacker);
                DeletePermanent(defender);
            }
        }

        private void ResolveSecurityBattle(Permanent attacker)
        {
            // 1. Check if Opponent has Security
            if (OpponentPlayer.Security.Count == 0)
            {
                Logger.Log($"[Battle] Direct Attack! {CurrentPlayer.Name} wins!");
                EndGame(CurrentPlayer);
                return;
            }

            // 2. Reveal Top Card
            Card securityCard = OpponentPlayer.Security[0];
            OpponentPlayer.Security.RemoveAt(0);
            Logger.Log($"[Security] Revealed: {securityCard.Name} ({securityCard.Id}) [DP: {securityCard.BaseDP}]");
            
            // Allow Effect Hook
            // TriggerSecurityRemoved(securityCard);

            // 3. Security Effect (Option/Tamer)
            if (securityCard.IsOption || securityCard.IsTamer)
            {
                 // Stub: Activate [Security] effect
                 Logger.LogVerbose($"[Security] Effect Activated (Stub).");
                 // Then add to hand or trash depending on card? Standard rule: usually Hand or Trash.
                 // Options usually Trash after effect. Tamers play.
                 OpponentPlayer.Trash.Add(securityCard); 
            }
            else if (securityCard.IsDigimon)
            {
                // 4. Security Battle (DP Only - Iceclad does not apply to Security Digimon)
                int attackerDp = attacker.CurrentDP;
                int securityDp = securityCard.BaseDP;

                Logger.LogVerbose($"[Security Battle] Attacker DP: {attackerDp} vs Security DP: {securityDp}");

                if (attackerDp < securityDp)
                {
                    Logger.Log($"[Security] Attacker Deleted by Security Digimon.");
                    DeletePermanent(attacker);
                    // Security digimon is discarded? Yes.
                }
                else
                {
                    Logger.Log($"[Security] Attacker survives.");
                }
                // Security Digimon always goes to trash after battle (unless effect says otherwise)
                OpponentPlayer.Trash.Add(securityCard);
            }
        }

        private void DeletePermanent(Permanent p)
        {
            Player owner = (p.TopCard.Id.StartsWith("P1") || Player1.BattleArea.Contains(p)) ? Player1 : Player2; 
            // Better owner check logic needed if we share IDs, but currently we rely on list containment
            if (Player1.BattleArea.Contains(p)) owner = Player1;
            else if (Player2.BattleArea.Contains(p)) owner = Player2;
            else return; // Already deleted?

            Logger.LogVerbose($"[Delete] Deleting {p.TopCard.Name} and {p.Sources.Count} sources.");
            
            // Move sources + top card to Trash
            List<Card> allCards = new List<Card>(p.Sources);
            allCards.Add(p.TopCard);
            owner.AddCardsToTrash(allCards);

            // Remove from Field
            owner.RemovePermanent(p);
        }

        public void ExecuteDigivolve(int handIndex, int fieldIndex)
        {
             // 1. Validate Phase
             if (TurnStateMachine.CurrentPhase != GamePhase.Main) return;

             // 2. Validate Indices
             if (handIndex < 0 || handIndex >= CurrentPlayer.Hand.Count) return;
             // Field Index 0-15 usually (ActionDecoder normalization)
             if (fieldIndex < 0 || fieldIndex >= CurrentPlayer.BattleArea.Count) return;

             Card evoCard = CurrentPlayer.Hand[handIndex];
             Permanent baseMon = CurrentPlayer.BattleArea[fieldIndex];

             // 3. Validate Logic
             // Level Check: New = Old + 1
             if (evoCard.Level != baseMon.TopCard.Level + 1)
             {
                 Logger.LogVerbose($"[Digivolve] Invalid Level: {baseMon.TopCard.Name} (Lv{baseMon.TopCard.Level}) -> {evoCard.Name} (Lv{evoCard.Level})");
                 return;
             }

             // Color Check: Must share at least one color
             bool colorMatch = false;
             foreach(var c in evoCard.Colors)
             {
                 if(baseMon.TopCard.Colors.Contains(c)) 
                 {
                     colorMatch = true;
                     break;
                 }
             }

             if (!colorMatch)
             {
                 Logger.LogVerbose($"[Digivolve] Invalid Color: Base {baseMon.TopCard.Colors[0]} vs Evo {evoCard.Colors[0]}");
                 return;
             }

             // 4. Pay Cost & Execute
             CurrentPlayer.Hand.RemoveAt(handIndex);
             PayCost(CurrentPlayer, evoCard.DigivolveCost);
             Logger.Log($"[Digivolve] {CurrentPlayer.Name} evolves {baseMon.TopCard.Name} into {evoCard.Name} (Cost: {evoCard.DigivolveCost})");
             
             baseMon.Digivolve(evoCard);

             // 5. Bonus Draw
             CurrentPlayer.Draw(Logger);

             // 6. Triggers (When Digivolving) - Stub
             // ActivateEffects(WhenDigivolving);

             // 7. Check Turn End
             TurnStateMachine.CheckTurnEnd();
        }
    }
}
