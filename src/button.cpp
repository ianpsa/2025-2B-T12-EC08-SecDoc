#include <WiFi.h>
#include <HTTPClient.h>

const char* ssid = "TPLinkBEA0";
const char* password = "unitree123";

const int button = 0;

void setup() {
  Serial.begin(115200);
  WiFi.begin(ssid, password);

  pinMode(button, INPUT_PULLUP);

  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");
  }
  Serial.println("Connected to Wi-Fi");
  Serial.print("IP Address: ");
  Serial.println(WiFi.localIP());
  Serial.print("Gateway (Router IP): ");
  Serial.println(WiFi.gatewayIP());  // Get router's IP address
}

void loop() {

  if (digitalRead(button)) {
    if (WiFi.isConnected()) {
      HTTPClient http;
      http.begin("http://192.168.0.43:3000/kill");
      http.addHeader("Content-Type", "application/json");

      String postBody = "{\"message\": \"SOCORROOOOOO!\"}";
      int httpResponseCode = http.POST(postBody);

      if (httpResponseCode > 0) {
        String responsePayload = http.getString();
        Serial.print("HTTP Response Code: ");
        Serial.println(httpResponseCode);
        Serial.print("Server Response: ");
        Serial.println(responsePayload);
      } else {
        Serial.print("Error sending POST request: ");
        Serial.println(httpResponseCode);
      }
      http.end();
    } else {
      Serial.println("WiFi Disconnected. Reconnecting...");
      WiFi.begin(ssid, password);  // Attempt to reconnect
    }
    delay(1000);
  }
}