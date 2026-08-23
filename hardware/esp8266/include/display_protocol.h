#pragma once

#include <Arduino.h>

namespace familiar {

constexpr uint8_t kDisplayProtocolVersion = 1;

enum class PetState : uint8_t {
  Idle = 0,
  Working,
  Thinking,
  Interacting,
  Happy,
  Celebrating,
  Alarmed,
  Sleeping,
  Watching,
};

struct DisplayState {
  PetState pet = PetState::Idle;
  uint16_t activeAgentCount = 0;
  uint64_t revision = 0;
  char serverId[37] = {};
};

enum class ParseResult : uint8_t {
  Accepted,
  IgnoredMessage,
  StaleRevision,
  InvalidJson,
  InvalidPayload,
  UnsupportedVersion,
};

PetState petStateForMood(const char* mood);
ParseResult parseDisplayMessage(const uint8_t* payload, size_t length,
                                DisplayState& current, bool& petChanged,
                                bool& countChanged);

}  // namespace familiar
