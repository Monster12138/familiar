#pragma once

#include <TFT_eSPI.h>

#include "display_protocol.h"

namespace familiar {

enum class ConnectionState : uint8_t { Connecting, Online, Offline };

class PetRenderer {
 public:
  explicit PetRenderer(TFT_eSPI& display) : display_(display) {}
  void begin();
  void drawPet(PetState state);
  void drawAgentCount(uint16_t count);
  void drawConnection(ConnectionState state);

 private:
  static constexpr uint16_t kBackground = 0x0861;
  static constexpr int16_t kSpriteX = 16;
  static constexpr int16_t kSpriteY = 16;
  TFT_eSPI& display_;
  uint16_t line_[96];
};

}  // namespace familiar
