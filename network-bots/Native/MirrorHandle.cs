using Microsoft.Win32.SafeHandles;

namespace MiniGameNetworkBot.Native;

internal sealed class MirrorHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    public MirrorHandle() : base(ownsHandle: true)
    {
    }

    internal MirrorHandle(nint handle) : base(ownsHandle: true)
    {
        SetHandle(handle);
    }

    protected override bool ReleaseHandle()
    {
        NativeMethods.mg_mirror_free(handle);
        return true;
    }
}
