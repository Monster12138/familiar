#include "display_protocol.h"

#include <ArduinoJson.h>
#include <cstring>

namespace familiar {

PetState petStateForMood(const char* mood) {
  if (mood == nullptr) return PetState::Idle;
  if (strcmp(mood, "Busy") == 0) return PetState::Working;
  if (strcmp(mood, "Thinking") == 0) return PetState::Thinking;
  if (strcmp(mood, "Interacting") == 0) return PetState::Interacting;
  if (strcmp(mood, "Celebrating") == 0) return PetState::Celebrating;
  if (strcmp(mood, "Alarmed") == 0) return PetState::Alarmed;
  if (strcmp(mood, "Sleepy") == 0) return PetState::Sleeping;
  if (strcmp(mood, "Watching") == 0) return PetState::Watching;
  return PetState::Idle;
}

ParseResult parseDisplayMessage(const uint8_t* payload, size_t length,
                                DisplayState& current, bool& petChanged,
                                bool& countChanged) {
  petChanged = false;
  countChanged = false;
  StaticJsonDocument<256> document;
  const auto error = deserializeJson(document, payload, length);
  if (error) return ParseResult::InvalidJson;

  const char* type = document["type"];
  if (type == nullptr || strcmp(type, "display") != 0) {
    return ParseResult::IgnoredMessage;
  }
  if (!document["v"].is<uint8_t>() ||
      document["v"].as<uint8_t>() != kDisplayProtocolVersion) {
    return ParseResult::UnsupportedVersion;
  }
  const char* serverId = document["server_id"];
  const char* mood = document["mood"];
  if (serverId == nullptr || strlen(serverId) != 36 || mood == nullptr ||
      !document["revision"].is<uint64_t>() ||
      !document["active_agent_count"].is<uint16_t>()) {
    return ParseResult::InvalidPayload;
  }

  const uint64_t revision = document["revision"].as<uint64_t>();
  const bool serverChanged = strcmp(current.serverId, serverId) != 0;
  if (!serverChanged && revision <= current.revision) {
    return ParseResult::StaleRevision;
  }

  const auto nextPet = petStateForMood(mood);
  const auto nextCount = document["active_agent_count"].as<uint16_t>();
  petChanged = serverChanged || nextPet != current.pet;
  countChanged = serverChanged || nextCount != current.activeAgentCount;
  current.pet = nextPet;
  current.activeAgentCount = nextCount;
  current.revision = revision;
  strlcpy(current.serverId, serverId, sizeof(current.serverId));
  return ParseResult::Accepted;
}

}  // namespace familiar
