import { create } from "@bufbuild/protobuf";

import { type CommonRefusal, type Refusal, RefusalSchema } from "@/proto/minigames/v2/refusal_pb";

/**
 * A refusal of the match itself rather than of a game's rules (paused, watching). Free of the
 * dictionaries, so the local authority's worker can use it without bundling them.
 */
export function commonRefusal(reason: CommonRefusal): Refusal {
  return create(RefusalSchema, { reason: { case: "common", value: reason } });
}
