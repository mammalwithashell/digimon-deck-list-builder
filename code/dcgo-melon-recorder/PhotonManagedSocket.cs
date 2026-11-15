using System;
using System.Collections.Concurrent;
using System.IO;
using System.Linq;
using System.Net.WebSockets;
using System.Reflection;
using System.Threading;
using System.Threading.Tasks;
using HarmonyLib;
using Il2CppExitGames.Client.Photon;
using Il2CppInterop.Runtime.InteropTypes.Arrays;
using MelonLoader;

namespace DcgoRecorder
{
    // Opt-in replacement for the standalone's IL2CPP ClientWebSocket on Wine.
    // Photon still owns serialization, authentication, rooms and all game traffic.
    // .NET's socket retains normal proxy discovery and certificate verification.
    internal static class PhotonManagedSocket
    {
        static bool installed;
        static readonly ConcurrentDictionary<IntPtr, Transport> sockets = new ConcurrentDictionary<IntPtr, Transport>();
        static readonly ConcurrentQueue<Action> callbacks = new ConcurrentQueue<Action>();

        public static void Install(HarmonyLib.Harmony harmony)
        {
            if (installed) return;
            foreach (var name in new[] { "AsyncConnectAndReceive", "Send", "Disconnect" })
                harmony.Patch(typeof(PhotonClientWebSocket).GetMethod(name,
                    BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic),
                    prefix: new HarmonyMethod(typeof(PhotonManagedSocket).GetMethod(name + "Prefix",
                        BindingFlags.Static | BindingFlags.NonPublic)));
            installed = true;
            MelonLogger.Msg("photon: managed WebSocket transport installed");
        }

        public static void Pump()
        {
            for (int i = 0; i < 128 && callbacks.TryDequeue(out var callback); i++)
                callback();
        }

        static bool AsyncConnectAndReceivePrefix(PhotonClientWebSocket __instance)
        {
            var transport = new Transport(__instance);
            sockets[__instance.Pointer] = transport;
            transport.Start(__instance.ConnectAddress, __instance.SerializationProtocol);
            return false;
        }

        static bool SendPrefix(PhotonClientWebSocket __instance, Il2CppStructArray<byte> __0,
            int __1, ref PhotonSocketError __result)
        {
            if (!sockets.TryGetValue(__instance.Pointer, out var transport)) return true;
            transport.Send(__0.Take(__1).ToArray());
            __result = PhotonSocketError.Success;
            return false;
        }

        static bool DisconnectPrefix(PhotonClientWebSocket __instance, ref bool __result)
        {
            if (!sockets.TryRemove(__instance.Pointer, out var transport)) return true;
            transport.Stop();
            __instance.State = PhotonSocketState.Disconnected;
            __result = true;
            return false;
        }

        sealed class Transport
        {
            readonly PhotonClientWebSocket owner;
            readonly ClientWebSocket socket = new ClientWebSocket();
            readonly CancellationTokenSource cancel = new CancellationTokenSource();
            readonly object sendGate = new object();
            Task sendTail = Task.CompletedTask;

            public Transport(PhotonClientWebSocket owner) { this.owner = owner; }
            public void Stop() { cancel.Cancel(); socket.Abort(); }

            public void Start(string address, string protocol)
            {
                var uri = new Uri(address);
                if (uri.Scheme != "wss") throw new ArgumentException("Managed Photon transport requires wss");
                MelonLogger.Msg("photon: managed connect " + uri.GetLeftPart(UriPartial.Path) + " protocol " + protocol);
                socket.Options.AddSubProtocol(protocol);
                _ = Run(address);
            }

            async Task Run(string address)
            {
                bool connected = false;
                try
                {
                    await socket.ConnectAsync(new Uri(address), cancel.Token).ConfigureAwait(false);
                    connected = true;
                    callbacks.Enqueue(() => {
                        if (cancel.IsCancellationRequested) return;
                        owner.State = PhotonSocketState.Connected;
                        owner.peerBase.OnConnect();
                        MelonLogger.Msg("photon: managed WebSocket connected");
                    });
                    var buffer = new byte[65536];
                    while (!cancel.IsCancellationRequested)
                    {
                        using (var message = new MemoryStream())
                        {
                            WebSocketReceiveResult received;
                            do
                            {
                                received = await socket.ReceiveAsync(new ArraySegment<byte>(buffer), cancel.Token).ConfigureAwait(false);
                                if (received.MessageType == WebSocketMessageType.Close)
                                    throw new IOException("Photon closed the WebSocket: " + received.CloseStatus);
                                if (received.MessageType != WebSocketMessageType.Binary)
                                    throw new IOException("Expected a binary Photon message");
                                if (message.Length + received.Count > 16 * 1024 * 1024)
                                    throw new IOException("Photon WebSocket message exceeds 16 MiB");
                                message.Write(buffer, 0, received.Count);
                            } while (!received.EndOfMessage);
                            var data = message.ToArray();
                            callbacks.Enqueue(() => {
                                if (!cancel.IsCancellationRequested)
                                    owner.HandleReceivedDatagram(new Il2CppStructArray<byte>(data), data.Length, false);
                            });
                        }
                    }
                }
                catch (Exception error)
                {
                    if (!cancel.IsCancellationRequested)
                        callbacks.Enqueue(() => {
                            if (cancel.IsCancellationRequested) return;
                            MelonLogger.Warning("photon: managed socket failed: " + error.Message);
                            owner.HandleException(connected ? StatusCode.ExceptionOnReceive : StatusCode.ExceptionOnConnect);
                        });
                }
                finally { socket.Dispose(); }
            }

            public void Send(byte[] data)
            {
                lock (sendGate)
                    sendTail = sendTail.ContinueWith(async previous => {
                        if (cancel.IsCancellationRequested) return;
                        try {
                            await socket.SendAsync(new ArraySegment<byte>(data), WebSocketMessageType.Binary,
                                true, cancel.Token).ConfigureAwait(false);
                        } catch (Exception error) {
                            if (!cancel.IsCancellationRequested)
                                callbacks.Enqueue(() => {
                                    if (cancel.IsCancellationRequested) return;
                                    MelonLogger.Warning("photon: managed send failed: " + error.Message);
                                    owner.HandleException(StatusCode.SendError);
                                });
                        }
                    }, TaskScheduler.Default).Unwrap();
            }
        }
    }
}
