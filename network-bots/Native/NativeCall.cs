using System.Text;
using Google.Protobuf;

namespace MiniGameNetworkBot.Native;

/// <summary>Status checking and buffer ownership for calls into the native core.</summary>
internal static unsafe class NativeCall
{
    private static int _abiChecked;

    /// <summary>Throws unless the loaded library speaks the ABI this code was written for.</summary>
    public static void EnsureCompatibleLibrary()
    {
        if (Volatile.Read(ref _abiChecked) == 1)
            return;
        var version = NativeMethods.mg_abi_version();
        if (version != NativeMethods.AbiVersion)
            throw new InvalidOperationException(
                $"mini_games native library has ABI {version}, this build needs {NativeMethods.AbiVersion}; rebuild mg-ffi");
        Volatile.Write(ref _abiChecked, 1);
    }

    public static void Check(MgStatus status)
    {
        if (status != MgStatus.Ok)
            throw new NativeCoreException(status, LastErrorMessage());
    }

    public static string LastErrorMessage()
    {
        MgBuffer buffer;
        return NativeMethods.mg_last_error_message(&buffer) == MgStatus.Ok
            ? Encoding.UTF8.GetString(Take(buffer))
            : "(no error message)";
    }

    /// <summary>Copies a library-filled buffer into managed memory and frees it.</summary>
    public static byte[] Take(MgBuffer buffer)
    {
        try
        {
            if (buffer.Data == 0 || buffer.Len == 0)
                return [];
            return new ReadOnlySpan<byte>((void*)buffer.Data, checked((int)buffer.Len)).ToArray();
        }
        finally
        {
            NativeMethods.mg_buffer_free(buffer);
        }
    }

    public static T Decode<T>(MgBuffer buffer, MessageParser<T> parser) where T : IMessage<T> =>
        parser.ParseFrom(Take(buffer));
}
