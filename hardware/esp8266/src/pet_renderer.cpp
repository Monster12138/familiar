#include "pet_renderer.h"

#include <pgmspace.h>

#include "generated/tabby_assets.h"

namespace familiar {

void PetRenderer::begin() {
  display_.init();
  display_.setRotation(0);
  display_.setSwapBytes(true);
  display_.fillScreen(kBackground);
  display_.setTextDatum(MC_DATUM);
  drawConnection(ConnectionState::Connecting);
  drawAgentCount(0);
  drawPet(PetState::Idle);
}

void PetRenderer::drawPet(PetState state) {
  assets::SpriteAsset sprite;
  memcpy_P(&sprite, &assets::kSprites[static_cast<uint8_t>(state)], sizeof(sprite));
  uint32_t offset = 0;
  for (uint16_t row = 0; row < assets::kSpriteHeight; ++row) {
    uint16_t column = 0;
    uint16_t outputColumn = 0;
    while (column < assets::kSpriteWidth && offset + 1 < sprite.length) {
      const uint8_t run = pgm_read_byte(sprite.data + offset++);
      const uint8_t paletteIndex = pgm_read_byte(sprite.data + offset++);
      const uint16_t color = paletteIndex == 0
                                 ? kBackground
                                 : pgm_read_word(&assets::kPalette[paletteIndex]);
      const uint16_t requestedEnd = column + run;
      const uint16_t end = requestedEnd < assets::kSpriteWidth
                               ? requestedEnd
                               : assets::kSpriteWidth;
      while (column < end) {
        if ((row & 1U) == 0 && (column & 1U) == 0 && outputColumn < 96) {
          line_[outputColumn++] = color;
        }
        ++column;
      }
    }
    if ((row & 1U) != 0) continue;
    while (outputColumn < 96) line_[outputColumn++] = kBackground;
    display_.pushImage(kSpriteX, kSpriteY + row / 2, 96, 1, line_);
    yield();
  }
}

void PetRenderer::drawAgentCount(uint16_t count) {
  display_.fillRect(0, 112, 128, 16, kBackground);
  display_.setTextColor(TFT_WHITE, kBackground);
  display_.setTextSize(1);
  char label[32];
  snprintf(label, sizeof(label), "AGENTS: %u", count);
  display_.drawString(label, 64, 120, 1);
}

void PetRenderer::drawConnection(ConnectionState state) {
  uint16_t color = TFT_ORANGE;
  const char* label = "CONNECTING";
  if (state == ConnectionState::Online) {
    color = TFT_GREEN;
    label = "ONLINE";
  } else if (state == ConnectionState::Offline) {
    color = TFT_RED;
    label = "OFFLINE";
  }
  display_.fillRect(0, 0, 128, 16, kBackground);
  display_.fillCircle(35, 8, 3, color);
  display_.setTextColor(TFT_WHITE, kBackground);
  display_.setTextSize(1);
  display_.drawString(label, 75, 8, 1);
}

}  // namespace familiar
