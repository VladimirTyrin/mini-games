using Minigames.V2;
using MiniGameNetworkBot.Native;
using MiniGameNetworkBot.Network;
using MiniGameNetworkBot.TicTacToe.Bots;
using MiniGameNetworkBot.TicTacToe.Core;

namespace MiniGameNetworkBot.TicTacToe;

/// <summary>
/// Hosts a TicTacToe lobby against a server-side bot and plays the match with an <see cref="IBot"/>.
/// The match is mirrored with the Rust core (lockstep): frames from the server are applied to a
/// <see cref="MatchMirror"/>, and the bot reads the mirror's view.
/// </summary>
public sealed class TicTacToeRunner(BotSession session, IBot bot, TicTacToeBotKind opponent)
{
    public const uint FieldSize = 15;
    public const uint WinCount = 5;

    /// <summary>Raised on every new state, on the runner's thread.</summary>
    public event Action<TicTacToeView, uint?>? ViewChanged;

    /// <summary>Raised once with the final view and whether this bot won.</summary>
    public event Action<TicTacToeView, MatchEnded>? MatchEnded;

    /// <summary>This bot's seat once the match started.</summary>
    public uint? MySlot { get; private set; }

    public async Task<MatchEnded> RunAsync(CancellationToken cancellationToken)
    {
        var header = await HostAsync(cancellationToken).ConfigureAwait(false);
        var ended = await PlayAsync(header, cancellationToken).ConfigureAwait(false);
        await LeaveLobbyAsync(cancellationToken).ConfigureAwait(false);
        return ended;
    }

    private async Task<MatchHeader> HostAsync(CancellationToken cancellationToken)
    {
        // A previous run of this bot account may still hold a seat (the server keeps it for the
        // resume grace period, and a new sign-in takes the session over).
        await LeaveLobbyAsync(cancellationToken).ConfigureAwait(false);
        var connection = session.Connection;
        var settings = new GameSettings
        {
            Tictactoe = new TicTacToeSettings
            {
                FieldWidth = FieldSize,
                FieldHeight = FieldSize,
                WinCount = WinCount,
                FirstPlayer = TicTacToeFirstPlayerMode.Random
            }
        };
        await connection.RequestAsync(new ClientMessage
        {
            CreateLobby = new CreateLobby { Name = $"{bot.Name} vs {opponent}", Settings = settings, MaxSeats = 2 }
        }, cancellationToken).ConfigureAwait(false);
        await connection.RequestAsync(new ClientMessage
        {
            AddBot = new AddBot { BotKind = new BotKind { Tictactoe = opponent } }
        }, cancellationToken).ConfigureAwait(false);
        await connection.RequestAsync(new ClientMessage { StartMatch = new StartMatch() }, cancellationToken)
            .ConfigureAwait(false);
        while (true)
        {
            var message = await connection.ReceiveAsync(cancellationToken).ConfigureAwait(false);
            if (message.MessageCase == ServerMessage.MessageOneofCase.MatchStarted)
                return message.MatchStarted;
        }
    }

    private Task LeaveLobbyAsync(CancellationToken cancellationToken) =>
        session.Connection.RequestAsync(new ClientMessage { LeaveLobby = new LeaveLobby() }, cancellationToken);

    private async Task<MatchEnded> PlayAsync(MatchHeader header, CancellationToken cancellationToken)
    {
        MySlot = session.SlotIn(header);
        var play = new MatchPlay(header, MySlot);
        try
        {
            var actFirst = true;
            while (true)
            {
                // A lost connection can surface on a send (a move) as well as on a receive: both
                // resume the seat the same way instead of ending the bot.
                try
                {
                    if (actFirst)
                    {
                        actFirst = false;
                        await ActAsync(play, cancellationToken).ConfigureAwait(false);
                    }
                    var message = await session.Connection.ReceiveAsync(cancellationToken).ConfigureAwait(false);
                    if (await HandleAsync(play, header, message, cancellationToken).ConfigureAwait(false) is { } ended)
                        return ended;
                }
                catch (ConnectionLostException)
                {
                    if (!await session.ReconnectAsync(cancellationToken).ConfigureAwait(false))
                        throw new InvalidOperationException("the server did not restore the seat after reconnecting");
                    // The server pushes a catch-up when the seat is resumed, or the lobby state when
                    // the match ended meanwhile.
                    play.WaitingForCatchUp = true;
                    play.SubmissionRequest = null;
                }
            }
        }
        finally
        {
            play.Mirror.Dispose();
        }
    }

    /// <summary>Applies one server message; returns the end of the match once it is over.</summary>
    private async Task<MatchEnded?> HandleAsync(
        MatchPlay play, MatchHeader header, ServerMessage message, CancellationToken cancellationToken)
    {
        switch (message.MessageCase)
        {
            case ServerMessage.MessageOneofCase.Frame:
                await OnFrameAsync(play, message.Frame, cancellationToken).ConfigureAwait(false);
                break;
            case ServerMessage.MessageOneofCase.CatchUp when message.CatchUp.Header?.MatchId == header.MatchId:
                await OnCatchUpAsync(play, message.CatchUp, cancellationToken).ConfigureAwait(false);
                break;
            case ServerMessage.MessageOneofCase.Ack when message.RequestId == play.SubmissionRequest:
                play.SubmissionRequest = null;
                play.AwaitingOwnFrame = true;
                break;
            case ServerMessage.MessageOneofCase.Error when message.RequestId == play.SubmissionRequest:
                Console.WriteLine($"Move refused: {message.Error.Code} {message.Error.Message}");
                play.SubmissionRequest = null;
                break;
            case ServerMessage.MessageOneofCase.Error when message.Error.Code == ErrorCode.SignedInElsewhere:
                throw new ServerErrorException(message.Error);
            case ServerMessage.MessageOneofCase.Paused when message.Paused.MatchId == header.MatchId:
                play.Paused = true;
                break;
            case ServerMessage.MessageOneofCase.Resumed when message.Resumed.MatchId == header.MatchId:
                await ResumeAfterAsync(play, message.Resumed.CountdownMs, cancellationToken).ConfigureAwait(false);
                break;
            case ServerMessage.MessageOneofCase.Ended when message.Ended.MatchId == header.MatchId:
                MatchEnded?.Invoke(View(play), message.Ended);
                return message.Ended;
            // The match ended while the connection was down: no Ended comes, the resumed lobby
            // reports the result instead.
            case ServerMessage.MessageOneofCase.LobbyState
                when play.WaitingForCatchUp && message.LobbyState.LastMatchId == header.MatchId:
            {
                var ended = new MatchEnded
                {
                    MatchId = header.MatchId,
                    Result = message.LobbyState.LastResult,
                    FinalTick = play.Mirror.NextTick,
                };
                MatchEnded?.Invoke(View(play), ended);
                return ended;
            }
        }
        return null;
    }

    private async Task ResumeAfterAsync(MatchPlay play, uint countdownMs, CancellationToken cancellationToken)
    {
        await Task.Delay((int)countdownMs, cancellationToken).ConfigureAwait(false);
        play.Paused = false;
        play.DecidedAt = null;
        await ActAsync(play, cancellationToken).ConfigureAwait(false);
    }

    private async Task OnFrameAsync(MatchPlay play, TickFrame frame, CancellationToken cancellationToken)
    {
        var nextTick = play.Mirror.NextTick;
        if (play.WaitingForCatchUp || frame.Tick < nextTick)
            return;
        if (frame.Tick > nextTick || !play.Mirror.TryApplyFrame(frame))
        {
            Console.WriteLine($"Desync at frame {frame.Tick}; asking for a catch-up");
            play.WaitingForCatchUp = true;
            await session.Connection.SendAsync(new ClientMessage
            {
                ReportDesync = new ReportDesync { MatchId = play.MatchId, Tick = frame.Tick, LocalHash = play.Mirror.StateHash }
            }, cancellationToken).ConfigureAwait(false);
            return;
        }
        play.Paused = false;
        if (frame.Inputs.Any(input => input.Command is { } command && command.Slot == play.MySlot))
        {
            play.SubmissionRequest = null;
            play.AwaitingOwnFrame = false;
        }
        await ActAsync(play, cancellationToken).ConfigureAwait(false);
    }

    private async Task OnCatchUpAsync(MatchPlay play, MatchCatchUp catchUp, CancellationToken cancellationToken)
    {
        var mirror = new MatchMirror(catchUp.Header);
        mirror.ApplyCatchUp(catchUp);
        play.Mirror.Dispose();
        play.Mirror = mirror;
        play.WaitingForCatchUp = false;
        play.SubmissionRequest = null;
        play.AwaitingOwnFrame = false;
        play.DecidedAt = null;
        play.Paused = catchUp.Pauses.Count > 0;
        // A catch-up sent during the resume countdown: no Resumed follows, so wait it out here.
        if (!play.Paused && catchUp.ResumeCountdownMs > 0)
            await ResumeAfterAsync(play, catchUp.ResumeCountdownMs, cancellationToken).ConfigureAwait(false);
        else if (!play.Paused)
            await ActAsync(play, cancellationToken).ConfigureAwait(false);
    }

    /// <summary>
    /// Asks the bot once per state. The server acknowledges a move before it sends the frame with
    /// it, so the bot is not asked again until that frame arrives.
    /// </summary>
    private async Task ActAsync(MatchPlay play, CancellationToken cancellationToken)
    {
        var view = View(play);
        ViewChanged?.Invoke(view, play.MySlot);
        var tick = play.Mirror.NextTick;
        if (play.MySlot is not { } me || play.WaitingForCatchUp || play.Paused
            || play.SubmissionRequest is not null || play.AwaitingOwnFrame
            || play.DecidedAt == tick || !view.HasCurrentSlot || view.CurrentSlot != me)
            return;
        play.DecidedAt = tick;
        var (x, y) = bot.GetMove(new BoardSnapshot(view));
        var command = new GameCommand
        {
            Tictactoe = new TicTacToeCommand { Place = new TicTacToePoint { X = (uint)x, Y = (uint)y } }
        };
        play.SubmissionRequest = await session.Connection.SendAsync(new ClientMessage
        {
            SubmitCommand = new SubmitCommand { MatchId = play.MatchId, Command = command }
        }, cancellationToken).ConfigureAwait(false);
    }

    private static TicTacToeView View(MatchPlay play) =>
        play.Mirror.View(play.MySlot is { } slot ? (int)slot : -1).Tictactoe;

    private sealed class MatchPlay(MatchHeader header, uint? mySlot)
    {
        public string MatchId { get; } = header.MatchId;
        public uint? MySlot { get; } = mySlot;
        public MatchMirror Mirror { get; set; } = new(header);
        public ulong? DecidedAt { get; set; }
        public uint? SubmissionRequest { get; set; }
        public bool AwaitingOwnFrame { get; set; }
        public bool WaitingForCatchUp { get; set; }
        public bool Paused { get; set; }
    }
}
