void setup() {
  pinMode(0, INPUT_PULLUP);  // Botão no GPIO 2
  Serial.begin(115200);
}

void loop() {
  int state = digitalRead(0);
  Serial.println(state == LOW ? "1" : "0");
  delay(50);
}