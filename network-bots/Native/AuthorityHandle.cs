using Microsoft.Win32.SafeHandles;

namespace MiniGameNetworkBot.Native;

internal sealed class AuthorityHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    public AuthorityHandle() : base(ownsHandle: true)
    {
    }

    internal AuthorityHandle(nint handle) : base(ownsHandle: true)
    {
        SetHandle(handle);
    }

    protected override bool ReleaseHandle()
    {
        NativeMethods.mg_authority_free(handle);
        return true;
    }
}
