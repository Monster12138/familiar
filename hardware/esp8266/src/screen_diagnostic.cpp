#include <Arduino.h>
#include <TFT_eSPI.h>

namespace {

TFT_eSPI display;
constexpr uint8_t kPossibleBacklightPins[] = {5, 16};
constexpr uint16_t kColors[] = {
    TFT_RED, TFT_GREEN, TFT_BLUE, TFT_WHITE, TFT_BLACK,
};
constexpr const char* kColorNames[] = {"RED", "GREEN", "BLUE", "WHITE", "BLACK"};
size_t colorIndex = 0;
uint32_t lastChange = 0;

}  // namespace

void setup() {
  Serial.begin(115200);
  Serial.println(F("Screen diagnostic starting"));
  for (const auto pin : kPossibleBacklightPins) {
    pinMode(pin, OUTPUT);
    digitalWrite(pin, HIGH);
  }
  display.init();
  display.setRotation(0);
  display.fillScreen(kColors[colorIndex]);
  Serial.printf("Display size: %dx%d; color: %s\n", display.width(), display.height(),
                kColorNames[colorIndex]);
  lastChange = millis();
}

void loop() {
  if (millis() - lastChange < 2000) return;
  colorIndex = (colorIndex + 1) % (sizeof(kColors) / sizeof(kColors[0]));
  display.fillScreen(kColors[colorIndex]);
  Serial.printf("Color: %s\n", kColorNames[colorIndex]);
  lastChange = millis();
}
