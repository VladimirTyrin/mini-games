namespace MiniGameNetworkBot.Network;

/// <summary>The WebSocket failed or closed; a reconnect with the resume token may recover.</summary>
public sealed class ConnectionLostException(string message, Exception? inner = null)
    : Exception(message, inner);
