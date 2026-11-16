namespace MiniGameNetworkBot.Native;

public sealed class NativeCoreException(MgStatus status, string message)
    : Exception($"{status}: {message}")
{
    public MgStatus Status { get; } = status;
}
