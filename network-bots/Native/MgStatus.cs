namespace MiniGameNetworkBot.Native;

/// <summary>Status codes of the native core; values match <c>MG_STATUS_*</c> in mini_games.h.</summary>
public enum MgStatus
{
    Ok = 0,
    NullArgument = 1,
    InvalidArgument = 2,
    Rejected = 3,
    Desync = 4,
    Internal = 5,
    InvalidState = 6,
}
