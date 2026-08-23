#include <Arduino.h>
#include <ESP8266WiFi.h>
#include <TFT_eSPI.h>
#include <WebSocketsClient.h>

#include "display_protocol.h"
#include "pet_renderer.h"
#include "secrets.h"

namespace {

constexpr char kDisplayPath[] = "/api/v1/display-stream";
constexpr uint32_t kReconnectInitialMs = 1000;
constexpr uint32_t kReconnectMaximumMs = 30000;
constexpr uint32_t kWifiRetryMs = 10000;

TFT_eSPI display;
WebSocketsClient socket;
familiar::PetRenderer renderer(display);
familiar::DisplayState state;
uint32_t reconnectMs = kReconnectInitialMs;
uint32_t lastWifiAttemptMs = 0;
String authorizationHeader;
bool socketStarted = false;
wl_status_t lastWifiStatus = WL_IDLE_STATUS;

void configureSocket() {
  if (FAMILIAR_TOKEN[0] != '\0') {
    authorizationHeader = String("Authorization: Bearer ") + FAMILIAR_TOKEN + "\r\n";
    socket.setExtraHeaders(authorizationHeader.c_str());
  } else {
    authorizationHeader = "";
    socket.setExtraHeaders(nullptr);
  }
  socket.setReconnectInterval(reconnectMs);
  socket.enableHeartbeat(30000, 5000, 2);
  socket.begin(FAMILIAR_HOST, FAMILIAR_PORT, kDisplayPath);
  socketStarted = true;
}

void onSocketEvent(WStype_t type, uint8_t* payload, size_t length) {
  switch (type) {
    case WStype_CONNECTED:
      reconnectMs = kReconnectInitialMs;
      socket.setReconnectInterval(reconnectMs);
      renderer.drawConnection(familiar::ConnectionState::Online);
      Serial.println(F("WebSocket connected"));
      break;
    case WStype_DISCONNECTED:
      renderer.drawConnection(familiar::ConnectionState::Offline);
      reconnectMs = min(kReconnectMaximumMs, reconnectMs * 2);
      socket.setReconnectInterval(reconnectMs);
      Serial.printf("WebSocket disconnected; retry in %u ms\n", reconnectMs);
      break;
    case WStype_TEXT: {
      bool petChanged = false;
      bool countChanged = false;
      const auto result = familiar::parseDisplayMessage(payload, length, state,
                                                        petChanged, countChanged);
      if (result == familiar::ParseResult::Accepted) {
        if (petChanged) renderer.drawPet(state.pet);
        if (countChanged) renderer.drawAgentCount(state.activeAgentCount);
      } else if (result != familiar::ParseResult::IgnoredMessage &&
                 result != familiar::ParseResult::StaleRevision) {
        Serial.printf("Rejected display message: %u\n", static_cast<uint8_t>(result));
      }
      break;
    }
    case WStype_ERROR:
      Serial.println(F("WebSocket error"));
      break;
    default:
      break;
  }
}

void ensureWifi() {
  const auto wifiStatus = WiFi.status();
  if (wifiStatus != lastWifiStatus) {
    Serial.printf("Wi-Fi status: %d\n", static_cast<int>(wifiStatus));
    lastWifiStatus = wifiStatus;
  }
  if (wifiStatus == WL_CONNECTED) {
    if (!socketStarted) {
      Serial.print(F("Wi-Fi connected, IP: "));
      Serial.println(WiFi.localIP());
      configureSocket();
    }
    return;
  }
  if (socketStarted) {
    socket.disconnect();
    socketStarted = false;
  }
  const uint32_t now = millis();
  if (now - lastWifiAttemptMs < kWifiRetryMs) return;
  lastWifiAttemptMs = now;
  renderer.drawConnection(familiar::ConnectionState::Connecting);
  WiFi.disconnect();
  WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
}

}  // namespace

void setup() {
  Serial.begin(115200);
  renderer.begin();
  WiFi.mode(WIFI_STA);
  WiFi.persistent(false);
  WiFi.setAutoReconnect(true);
  WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
  lastWifiAttemptMs = millis();
  socket.onEvent(onSocketEvent);
}

void loop() {
  ensureWifi();
  socket.loop();
  delay(1);
}
