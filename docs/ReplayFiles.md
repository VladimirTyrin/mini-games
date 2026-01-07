# Replay files format

We describe version 1 here. Future versions may be added

Replay file has an extension of .minigamesreplay
It captures 2 main things:
1. RNG seed
2. Player turns

## Binary format

First byte of the file is VERSION. It MUST be equal to 1

Next goes single protobuf-encoded message.
```
// game_service.proto
enum Game {
	GAME_UNSPECIFIED = 0;
	GAME_SNAKE = 1;
	GAME_TICTACTOE = 2;
}

// game_service.proto
message PlayerIdentity {
    string player_id = 1;
    bool is_bot = 2;
}

// replay.proto
message PlayerDisconnected { }

// replay.proto
message PlayerActionContent {
	oneof content {
		PlayerDisconnected disconnected = 1;
		InGameCommand command = 2;
	}
}

// replay.proto
message PlayerAction {
	int64 tick = 1; // Tick when the action should be applied
	int32 player_index = 2; // Player index in ReplayV1.players
	PlayerActionContent content = 3;
}

// replay.proto
message ReplayV1 {
	string engine_version = 1; // Game version game was played on
	int64 game_started_timestamp_ms = 2; // Millisecond-presicion Unix timestamp
	Game game = 3; // We don't rely on further oneof's so we can read replay header and display info
	uint64 seed = 4; // RNG seed. Note that reordering RNG operation in game engine becomes breaking replay change. This order SHOULD be well-defined
	LobbySettings lobby_settings = 5; // Game-specific lobby settings
	repeated PlayerIdentity players = 6; // Players. Moves will refer to players by index, so order matters
	
	reserved 7-9; // Reserved for replay header extensions
	
	repeated PlayerAction actions = 10;
}
```

Main con of this approach is the nature of protobuf parsers (and the format itself) - they don't really support streamed parsing. We may add custom streamable binary format for version 2 (it will require custom serialization and deserialization logic)

Note that we can define ReplayV1Header message without actions field and deserialize it to show replay metadata without storing the whole file in memory