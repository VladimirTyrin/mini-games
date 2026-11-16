using System.Net.WebSockets;
using System.Text;
using Google.Protobuf;
using Minigames.V2;

namespace MiniGameNetworkBot.Network;

/// <summary>
/// One WebSocket to the server carrying protobuf JSON (protocol v2, see docs/protocol.md).
/// Single reader: replies to <see cref="RequestAsync"/> are matched by request id, and pushes
/// that arrive meanwhile are queued for <see cref="ReceiveAsync"/>. Pings every 15 s keep the
/// connection alive (the server closes silent connections after 45 s).
/// </summary>
public sealed class ServerConnection : IAsyncDisposable
{
    private static readonly TimeSpan PingEvery = TimeSpan.FromSeconds(15);
    private static readonly JsonFormatter Formatter = new(JsonFormatter.Settings.Default);
    private static readonly JsonParser Parser = new(JsonParser.Settings.Default.WithIgnoreUnknownFields(true));

    private readonly ClientWebSocket _socket;
    private readonly SemaphoreSlim _sendLock = new(1, 1);
    private readonly Queue<ServerMessage> _pending = new();
    private readonly CancellationTokenSource _stopPinging = new();
    private readonly Task _pingLoop;
    private uint _nextRequestId = 1;
    private bool _disposed;

    private ServerConnection(ClientWebSocket socket)
    {
        _socket = socket;
        _pingLoop = PingLoopAsync(_stopPinging.Token);
    }

    public static async Task<ServerConnection> ConnectAsync(Uri url, CancellationToken cancellationToken)
    {
        var socket = new ClientWebSocket();
        try
        {
            await socket.ConnectAsync(url, cancellationToken).ConfigureAwait(false);
        }
        catch (Exception exception) when (exception is WebSocketException or HttpRequestException)
        {
            socket.Dispose();
            throw new ConnectionLostException("cannot connect to " + url, exception);
        }
        return new ServerConnection(socket);
    }

    /// <summary>Sends without waiting; returns the request id the reply will carry.</summary>
    public async Task<uint> SendAsync(ClientMessage message, CancellationToken cancellationToken)
    {
        await _sendLock.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            message.RequestId = _nextRequestId++;
            await SendLockedAsync(message, cancellationToken).ConfigureAwait(false);
            return message.RequestId;
        }
        finally
        {
            _sendLock.Release();
        }
    }

    /// <summary>Sends and waits for the reply; an error reply throws <see cref="ServerErrorException"/>.</summary>
    public async Task<ServerMessage> RequestAsync(ClientMessage message, CancellationToken cancellationToken)
    {
        var requestId = await SendAsync(message, cancellationToken).ConfigureAwait(false);
        while (true)
        {
            var reply = await ReadAsync(cancellationToken).ConfigureAwait(false);
            if (reply.RequestId != requestId)
            {
                _pending.Enqueue(reply);
                continue;
            }
            if (reply.MessageCase == ServerMessage.MessageOneofCase.Error)
                throw new ServerErrorException(reply.Error);
            return reply;
        }
    }

    /// <summary>The next server message, queued pushes first.</summary>
    public Task<ServerMessage> ReceiveAsync(CancellationToken cancellationToken) =>
        _pending.TryDequeue(out var message) ? Task.FromResult(message) : ReadAsync(cancellationToken);

    private async Task<ServerMessage> ReadAsync(CancellationToken cancellationToken)
    {
        var buffer = new byte[16 * 1024];
        using var text = new MemoryStream();
        while (true)
        {
            WebSocketReceiveResult result;
            try
            {
                result = await _socket.ReceiveAsync(buffer, cancellationToken).ConfigureAwait(false);
            }
            catch (WebSocketException exception)
            {
                throw new ConnectionLostException("connection lost", exception);
            }
            if (result.MessageType == WebSocketMessageType.Close)
                throw new ConnectionLostException("the server closed the connection");
            text.Write(buffer, 0, result.Count);
            if (!result.EndOfMessage)
                continue;
            if (result.MessageType != WebSocketMessageType.Text)
            {
                text.SetLength(0);
                continue;
            }
            return Parser.Parse<ServerMessage>(Encoding.UTF8.GetString(text.GetBuffer(), 0, (int)text.Length));
        }
    }

    private async Task SendLockedAsync(ClientMessage message, CancellationToken cancellationToken)
    {
        var bytes = Encoding.UTF8.GetBytes(Formatter.Format(message));
        try
        {
            await _socket.SendAsync(bytes, WebSocketMessageType.Text, endOfMessage: true, cancellationToken)
                .ConfigureAwait(false);
        }
        catch (WebSocketException exception)
        {
            throw new ConnectionLostException("connection lost", exception);
        }
    }

    private async Task PingLoopAsync(CancellationToken cancellationToken)
    {
        using var timer = new PeriodicTimer(PingEvery);
        try
        {
            while (await timer.WaitForNextTickAsync(cancellationToken).ConfigureAwait(false))
            {
                var ping = new ClientMessage { Ping = new Ping { ClientTimeMs = (ulong)Environment.TickCount64 } };
                await SendAsync(ping, cancellationToken).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
        }
        catch (ConnectionLostException)
        {
            // The reader notices the lost connection too and reports it.
        }
    }

    public async ValueTask DisposeAsync()
    {
        if (_disposed)
            return;
        _disposed = true;
        await _stopPinging.CancelAsync().ConfigureAwait(false);
        await _pingLoop.ConfigureAwait(ConfigureAwaitOptions.SuppressThrowing);
        if (_socket.State == WebSocketState.Open)
        {
            try
            {
                using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(2));
                await _socket.CloseAsync(WebSocketCloseStatus.NormalClosure, "bye", timeout.Token).ConfigureAwait(false);
            }
            catch (Exception exception) when (exception is WebSocketException or OperationCanceledException)
            {
                // Closing politely is a courtesy; the socket is disposed either way.
            }
        }
        _socket.Dispose();
        _stopPinging.Dispose();
        _sendLock.Dispose();
    }
}
