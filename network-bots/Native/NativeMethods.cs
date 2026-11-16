using System.Runtime.InteropServices;

namespace MiniGameNetworkBot.Native;

/// <summary>Declarations of crates/ffi/include/mini_games.h.</summary>
internal static unsafe partial class NativeMethods
{
    private const string Library = "mini_games";

    /// <summary>The <c>MG_ABI_VERSION</c> this code was written against.</summary>
    internal const uint AbiVersion = 1;

    [LibraryImport(Library)]
    internal static partial uint mg_abi_version();

    [LibraryImport(Library)]
    internal static partial MgStatus mg_last_error_message(MgBuffer* outMessage);

    [LibraryImport(Library)]
    internal static partial void mg_buffer_free(MgBuffer buffer);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_default_settings(byte* gameId, nuint gameIdLen, MgBuffer* outSettings);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_validate_settings(byte* settings, nuint settingsLen, uint seats);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_authority_new(byte* header, nuint headerLen, ulong seed, nint* outAuthority);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_authority_clone(AuthorityHandle authority, nint* outAuthority);

    [LibraryImport(Library)]
    internal static partial void mg_authority_free(nint authority);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_authority_submit(AuthorityHandle authority, uint slot, byte* command, nuint commandLen);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_authority_run_bots(AuthorityHandle authority);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_authority_seal_frame(AuthorityHandle authority, MgBuffer* outFrame);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_authority_is_waiting_for_bot(AuthorityHandle authority, byte* outWaiting);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_authority_view(AuthorityHandle authority, int viewerSlot, MgBuffer* outView);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_authority_result(AuthorityHandle authority, MgBuffer* outResult);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_authority_replay_file(AuthorityHandle authority, MgBuffer* outReplay);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_mirror_new(byte* header, nuint headerLen, nint* outMirror);

    [LibraryImport(Library)]
    internal static partial void mg_mirror_free(nint mirror);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_mirror_apply_frame(MirrorHandle mirror, byte* frame, nuint frameLen);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_mirror_apply_catch_up(MirrorHandle mirror, byte* catchUp, nuint catchUpLen);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_mirror_next_tick(MirrorHandle mirror, ulong* outTick);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_mirror_is_finished(MirrorHandle mirror, byte* outFinished);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_mirror_state_hash(MirrorHandle mirror, ulong* outHash);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_mirror_view(MirrorHandle mirror, int viewerSlot, MgBuffer* outView);

    [LibraryImport(Library)]
    internal static partial MgStatus mg_mirror_result(MirrorHandle mirror, MgBuffer* outResult);
}
