using Minigames.V2;

namespace MiniGameNetworkBot.Network;

public sealed class ServerErrorException(ErrorResponse error)
    : Exception($"{error.Code}: {error.Message}")
{
    public ErrorCode Code { get; } = error.Code;
}
