/*
 * mini_games: C ABI over the mini-games core (crate mg-ffi, library `mini_games`).
 *
 * Messages are binary protobuf of proto/minigames/v2 (MatchHeader, GameCommand, TickFrame,
 * MatchCatchUp, GameView, GameSettings, MatchResult, ReplayFile, BotKind).
 *
 * Contract for every function:
 *  - The return value is an MgStatus. After a failure, mg_last_error_message() describes it
 *    (per thread, kept until the next failure).
 *  - Input bytes are (pointer, length) pairs; the pointer may be NULL only when the length is 0.
 *    Out-pointers must point to writable memory of their type.
 *  - Handles come from a *_new, *_clone or *_from_* function and are released exactly once with
 *    the matching *_free; a handle must not be used by two threads at the same time.
 *  - An MgBuffer filled by the library belongs to the caller until mg_buffer_free(). Its data is
 *    NULL when the result is empty.
 *  - Panics never unwind into the caller: they become MG_STATUS_INTERNAL, after which the handle
 *    involved should be freed rather than used.
 *
 * Kept in sync with the Rust sources by crates/ffi/tests/header.rs.
 */
#ifndef MINI_GAMES_H
#define MINI_GAMES_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define MG_ABI_VERSION 1

typedef int32_t MgStatus;
#define MG_STATUS_OK 0
#define MG_STATUS_NULL_ARGUMENT 1
/* Undecodable bytes, invalid settings or header, unknown game, slot out of range. */
#define MG_STATUS_INVALID_ARGUMENT 2
/* The game refused the command (not your turn, illegal move, game over); nothing changed. */
#define MG_STATUS_REJECTED 3
/* A frame or catch-up does not apply: the mirror is out of sync; rebuild it from a catch-up. */
#define MG_STATUS_DESYNC 4
/* A bug in the library (including a caught panic); free the handle involved. */
#define MG_STATUS_INTERNAL 5
/* The call does not fit the handle (mg_mirror_advance_to without a replay log). */
#define MG_STATUS_INVALID_STATE 6

typedef struct MgBuffer {
    uint8_t *data;
    size_t len;
    size_t capacity;
} MgBuffer;

/* A match run locally: authority plus a mirror of it for views. */
typedef struct MgAuthority MgAuthority;
/* Receiving side of lockstep: frames from a server, or a replay file. */
typedef struct MgMirror MgMirror;

uint32_t mg_abi_version(void);
MgStatus mg_last_error_message(MgBuffer *out_message);
void mg_buffer_free(MgBuffer buffer);

/* Catalog. game_id is UTF-8 ("tictactoe", "snake", ...). */
MgStatus mg_default_settings(const uint8_t *game_id, size_t game_id_len, MgBuffer *out_settings);
/* Each BotKind encoded length-delimited, concatenated. */
MgStatus mg_bot_kinds(const uint8_t *game_id, size_t game_id_len, MgBuffer *out_bot_kinds);
MgStatus mg_validate_settings(const uint8_t *settings, size_t settings_len, uint32_t seats);

/* Local authority. A seat with bot_kind in the header is played by that built-in bot. */
MgStatus mg_authority_new(const uint8_t *header, size_t header_len, uint64_t seed,
                          MgAuthority **out_authority);
/* Independent copy in the same state; costs time proportional to the match so far. */
MgStatus mg_authority_clone(const MgAuthority *authority, MgAuthority **out_authority);
void mg_authority_free(MgAuthority *authority);
MgStatus mg_authority_submit(MgAuthority *authority, uint32_t slot, const uint8_t *command,
                             size_t command_len);
MgStatus mg_authority_drop_player(MgAuthority *authority, uint32_t slot);
MgStatus mg_authority_run_bots(MgAuthority *authority);
MgStatus mg_authority_seal_frame(MgAuthority *authority, MgBuffer *out_frame);
MgStatus mg_authority_is_waiting_for_bot(const MgAuthority *authority, bool *out_waiting);
MgStatus mg_authority_has_pending_inputs(const MgAuthority *authority, bool *out_pending);
MgStatus mg_authority_is_finished(const MgAuthority *authority, bool *out_finished);
/* 0 for turn-based games. */
MgStatus mg_authority_tick_ms(const MgAuthority *authority, uint32_t *out_tick_ms);
MgStatus mg_authority_next_tick(const MgAuthority *authority, uint64_t *out_tick);
/* viewer_slot < 0 renders for a spectator. */
MgStatus mg_authority_view(const MgAuthority *authority, int32_t viewer_slot, MgBuffer *out_view);
/* Empty buffer while the game runs. */
MgStatus mg_authority_result(const MgAuthority *authority, MgBuffer *out_result);
MgStatus mg_authority_replay_file(const MgAuthority *authority, MgBuffer *out_replay);

/* Mirror. */
MgStatus mg_mirror_new(const uint8_t *header, size_t header_len, MgMirror **out_mirror);
MgStatus mg_mirror_from_replay_file(const uint8_t *replay, size_t replay_len,
                                    MgMirror **out_mirror);
void mg_mirror_free(MgMirror *mirror);
MgStatus mg_mirror_apply_frame(MgMirror *mirror, const uint8_t *frame, size_t frame_len);
MgStatus mg_mirror_apply_catch_up(MgMirror *mirror, const uint8_t *catch_up,
                                  size_t catch_up_len);
MgStatus mg_mirror_advance_to(MgMirror *mirror, uint64_t stop_tick);
MgStatus mg_mirror_replay_end_tick(const MgMirror *mirror, uint64_t *out_tick);
MgStatus mg_mirror_next_tick(const MgMirror *mirror, uint64_t *out_tick);
MgStatus mg_mirror_is_finished(const MgMirror *mirror, bool *out_finished);
MgStatus mg_mirror_state_hash(const MgMirror *mirror, uint64_t *out_hash);
MgStatus mg_mirror_view(const MgMirror *mirror, int32_t viewer_slot, MgBuffer *out_view);
MgStatus mg_mirror_result(const MgMirror *mirror, MgBuffer *out_result);

#ifdef __cplusplus
}
#endif

#endif /* MINI_GAMES_H */
