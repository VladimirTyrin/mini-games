using System.Runtime.InteropServices;

namespace MiniGameNetworkBot.Native;

/// <summary>Bytes owned by the native library until <c>mg_buffer_free</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct MgBuffer
{
    public nint Data;
    public nuint Len;
    public nuint Capacity;
}
