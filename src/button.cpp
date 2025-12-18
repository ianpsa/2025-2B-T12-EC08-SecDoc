#include <WiFi.h>
#include <HTTPClient.h>

const char *endpointBase = "http://10.8.250.18:3000/emergency";

const char *ssid = "InteliRobo";
const char *password = "12345678";

const int button = 6;

bool lastState = HIGH;
unsigned long lastDebounce = 0;
const unsigned long debounceTime = 80; // ajuste fino

void setup() {
    Serial.begin(115200);

    WiFi.begin(ssid, password);
    while (WiFi.status() != WL_CONNECTED) {
        delay(500);
        Serial.print(".");
    }
    Serial.println("\nWiFi conectado!");


    pinMode(button, INPUT_PULLUP);
}

void loop() {

    // lê o botão
    int currentState = digitalRead(button);

    // Debounce manual
    if (currentState != lastState) {
        lastDebounce = millis();
        lastState = currentState;

        delay(10); // Estabilização rápida opcional
    }

    if (millis() - lastDebounce > debounceTime) {

        // Botão apertado (HIGH → LOW)
        if (currentState == LOW) {

            HTTPClient http;
            http.begin(String(endpointBase) + "/release");
            http.addHeader("Content-Type", "application/json");
            int code = http.POST("{}");
            http.end();

            Serial.println("Kill enviado. Code: " + String(code));

            // Evita múltiplos envios enquanto mantém pressionado
            while (digitalRead(button) == LOW) {
                delay(10);
            }


            HTTPClient http2;
            http2.begin(String(endpointBase) + "/press");
            http2.addHeader("Content-Type", "application/json");
            code = http2.POST("{}");
            http2.end();
        }
    }

    delay(5); // pequena folga
}
