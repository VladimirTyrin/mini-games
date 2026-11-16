using Google.Protobuf;
using Minigames.V2;

namespace MiniGameNetworkBot.Native;

/// <summary>
/// A match run locally by the Rust core (any game of the registry): the core validates every
/// command, plays built-in bot seats and seals frames. Not thread-safe; use one per thread.
/// </summary>
public sealed unsafe class LocalMatch : IDisposable
{
    private readonly AuthorityHandle _handle;

    /// <param name="header">Settings and seats; a seat with <c>BotKind</c> is played by that core bot.</param>
    /// <param name="seed">Drives all randomness of the match.</param>
    public LocalMatch(MatchHeader header, ulong seed)
    {
        NativeCall.EnsureCompatibleLibrary();
        var bytes = header.ToByteArray();
        nint handle;
        fixed (byte* data = bytes)
            NativeCall.Check(NativeMethods.mg_authority_new(data, (nuint)bytes.Length, seed, &handle));
        _handle = new AuthorityHandle(handle);
    }

    private LocalMatch(AuthorityHandle handle) => _handle = handle;

    /// <summary>Returns false when the game refuses the command (illegal now); nothing changes then.</summary>
    public bool TrySubmit(uint slot, GameCommand command)
    {
        var bytes = command.ToByteArray();
        MgStatus status;
        fixed (byte* data = bytes)
            status = NativeMethods.mg_authority_submit(_handle, slot, data, (nuint)bytes.Length);
        if (status == MgStatus.Rejected)
            return false;
        NativeCall.Check(status);
        return true;
    }

    public void RunBots() => NativeCall.Check(NativeMethods.mg_authority_run_bots(_handle));

    public bool IsWaitingForBot
    {
        get
        {
            byte waiting;
            NativeCall.Check(NativeMethods.mg_authority_is_waiting_for_bot(_handle, &waiting));
            return waiting != 0;
        }
    }

    public TickFrame SealFrame()
    {
        MgBuffer frame;
        NativeCall.Check(NativeMethods.mg_authority_seal_frame(_handle, &frame));
        return NativeCall.Decode(frame, TickFrame.Parser);
    }

    /// <param name="viewerSlot">Negative for a spectator.</param>
    public GameView View(int viewerSlot = -1)
    {
        MgBuffer view;
        NativeCall.Check(NativeMethods.mg_authority_view(_handle, viewerSlot, &view));
        return NativeCall.Decode(view, GameView.Parser);
    }

    /// <summary>Null while the game runs.</summary>
    public MatchResult? Result
    {
        get
        {
            MgBuffer result;
            NativeCall.Check(NativeMethods.mg_authority_result(_handle, &result));
            var bytes = NativeCall.Take(result);
            return bytes.Length == 0 ? null : MatchResult.Parser.ParseFrom(bytes);
        }
    }

    public ReplayFile ReplayFile()
    {
        MgBuffer replay;
        NativeCall.Check(NativeMethods.mg_authority_replay_file(_handle, &replay));
        return NativeCall.Decode(replay, Minigames.V2.ReplayFile.Parser);
    }

    /// <summary>An independent copy in the same state; costs time proportional to the match so far.</summary>
    public LocalMatch Clone()
    {
        nint handle;
        NativeCall.Check(NativeMethods.mg_authority_clone(_handle, &handle));
        return new LocalMatch(new AuthorityHandle(handle));
    }

    public void Dispose() => _handle.Dispose();
}
