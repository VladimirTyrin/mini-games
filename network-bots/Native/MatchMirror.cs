using Google.Protobuf;
using Minigames.V2;

namespace MiniGameNetworkBot.Native;

/// <summary>
/// The receiving side of lockstep: replays the server's frames with the same Rust core the
/// server runs, so the bot sees exactly the server's state. Not thread-safe.
/// </summary>
public sealed unsafe class MatchMirror : IDisposable
{
    private readonly MirrorHandle _handle;

    public MatchMirror(MatchHeader header)
    {
        NativeCall.EnsureCompatibleLibrary();
        var bytes = header.ToByteArray();
        nint handle;
        fixed (byte* data = bytes)
            NativeCall.Check(NativeMethods.mg_mirror_new(data, (nuint)bytes.Length, &handle));
        _handle = new MirrorHandle(handle);
    }

    /// <summary>Returns false on a desync (gap, rejected input, hash mismatch): rebuild from a catch-up.</summary>
    public bool TryApplyFrame(TickFrame frame)
    {
        var bytes = frame.ToByteArray();
        MgStatus status;
        fixed (byte* data = bytes)
            status = NativeMethods.mg_mirror_apply_frame(_handle, data, (nuint)bytes.Length);
        if (status == MgStatus.Desync)
            return false;
        NativeCall.Check(status);
        return true;
    }

    /// <summary>Applies a catch-up to a mirror created from its header.</summary>
    public void ApplyCatchUp(MatchCatchUp catchUp)
    {
        var bytes = catchUp.ToByteArray();
        fixed (byte* data = bytes)
            NativeCall.Check(NativeMethods.mg_mirror_apply_catch_up(_handle, data, (nuint)bytes.Length));
    }

    public ulong NextTick
    {
        get
        {
            ulong tick;
            NativeCall.Check(NativeMethods.mg_mirror_next_tick(_handle, &tick));
            return tick;
        }
    }

    public bool IsFinished
    {
        get
        {
            byte finished;
            NativeCall.Check(NativeMethods.mg_mirror_is_finished(_handle, &finished));
            return finished != 0;
        }
    }

    public ulong StateHash
    {
        get
        {
            ulong hash;
            NativeCall.Check(NativeMethods.mg_mirror_state_hash(_handle, &hash));
            return hash;
        }
    }

    /// <param name="viewerSlot">Negative for a spectator.</param>
    public GameView View(int viewerSlot)
    {
        MgBuffer view;
        NativeCall.Check(NativeMethods.mg_mirror_view(_handle, viewerSlot, &view));
        return NativeCall.Decode(view, GameView.Parser);
    }

    public void Dispose() => _handle.Dispose();
}
