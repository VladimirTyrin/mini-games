using Minigames.V2;

namespace MiniGameNetworkBot.Network;

/// <summary>
/// A connection signed in with a bot key. <see cref="ReconnectAsync"/> opens a new connection
/// and reclaims the lobby (and match) seat with the resume token.
/// </summary>
public sealed class BotSession : IAsyncDisposable
{
    public const uint ProtocolVersion = 2;

    private readonly Uri _url;
    private readonly string _botKey;

    private BotSession(Uri url, string botKey, ServerConnection connection, Welcome welcome)
    {
        _url = url;
        _botKey = botKey;
        Connection = connection;
        Welcome = welcome;
    }

    public ServerConnection Connection { get; private set; }
    public Welcome Welcome { get; private set; }
    public Identity Identity => Welcome.Identity;

    public static async Task<BotSession> ConnectAsync(Uri url, string botKey, CancellationToken cancellationToken)
    {
        var (connection, welcome) = await SignInAsync(url, botKey, resumeToken: "", cancellationToken).ConfigureAwait(false);
        return new BotSession(url, botKey, connection, welcome);
    }

    /// <summary>Returns whether the server gave the seat back (false: the old session was gone).</summary>
    public async Task<bool> ReconnectAsync(CancellationToken cancellationToken)
    {
        await Connection.DisposeAsync().ConfigureAwait(false);
        var delay = TimeSpan.Zero;
        for (var attempt = 1; ; attempt++)
        {
            await Task.Delay(delay, cancellationToken).ConfigureAwait(false);
            try
            {
                var (connection, welcome) = await SignInAsync(_url, _botKey, Welcome.ResumeToken, cancellationToken)
                    .ConfigureAwait(false);
                Connection = connection;
                Welcome = welcome;
                return welcome.Resumed;
            }
            catch (ConnectionLostException) when (attempt < 8)
            {
                delay = delay == TimeSpan.Zero ? TimeSpan.FromMilliseconds(250) : TimeSpan.FromTicks(Math.Min(delay.Ticks * 2, TimeSpan.FromSeconds(8).Ticks));
            }
        }
    }

    /// <summary>This bot's seat in a match; null when it only watches.</summary>
    public uint? SlotIn(MatchHeader header)
    {
        foreach (var seat in header.Seats)
        {
            if (seat.Player is { } player && player.Kind == Identity.Kind && player.Id == Identity.Id)
                return seat.Slot;
        }
        return null;
    }

    private static async Task<(ServerConnection Connection, Welcome Welcome)> SignInAsync(
        Uri url, string botKey, string resumeToken, CancellationToken cancellationToken)
    {
        var connection = await ServerConnection.ConnectAsync(url, cancellationToken).ConfigureAwait(false);
        try
        {
            var hello = new Hello
            {
                ProtocolVersion = ProtocolVersion,
                ClientVersion = "MiniGameNetworkBot/" + VersionInfo.ClientVersion,
                BotClient = new BotClientCredentials { Key = botKey },
                ResumeToken = resumeToken
            };
            var reply = await connection.RequestAsync(new ClientMessage { Hello = hello }, cancellationToken)
                .ConfigureAwait(false);
            if (reply.MessageCase != ServerMessage.MessageOneofCase.Welcome)
                throw new InvalidOperationException($"expected Welcome, got {reply.MessageCase}");
            return (connection, reply.Welcome);
        }
        catch
        {
            await connection.DisposeAsync().ConfigureAwait(false);
            throw;
        }
    }

    public ValueTask DisposeAsync() => Connection.DisposeAsync();
}
