# **Go2 Emojis - Endpoint implementation**

Below is a description of each mapped API constant, what it makes the robot do, and the corresponding HTTP endpoint.

---

## 🇺🇸 **English — API Definitions**

| Constant                     | ID   | Endpoint   | Action Description                                                                     |
| ---------------------------- | ---- | ---------- | -------------------------------------------------------------------------------------- |
| `ROBOT_SPORT_API_ID_HELLO`   | 1016 | `/hello`   | Robot performs a friendly "hello" gesture — usually a front-leg wave or small bow.     |
| `ROBOT_SPORT_API_ID_STRETCH` | 1017 | `/stretch` | Robot executes a full-body stretch routine to loosen joints and prepare motors.        |
| `ROBOT_SPORT_API_ID_CONTENT` | 1020 | `/content` | Robot expresses a “happy/content” behavior — relaxed stance, tail wag (LED animation). |
| `ROBOT_SPORT_API_ID_WALLOW`  | 1021 | `/wallow`  | Robot rolls or leans side-to-side like a playful "wallow".                             |
| `ROBOT_SPORT_API_ID_DANCE1`  | 1022 | `/dance1`  | Robot performs dance routine #1 (fast rhythmic dance).                                 |
| `ROBOT_SPORT_API_ID_DANCE2`  | 1023 | `/dance2`  | Robot performs dance routine #2 (slower or alternate choreography).                    |
| `ROBOT_SPORT_API_ID_POSE`    | 1028 | `/pose`    | Robot transitions to a pose stance — static stable position for photos or demos.       |
| `ROBOT_SPORT_API_ID_SCRAPE`  | 1029 | `/scrape`  | Robot performs a ground “scrape” gesture — paw scraping motion like a bull preparing.  |

---

### 🔧 **Example: Triggering an action via curl**

Replace `<bot_ip>` with your Go2 robot’s real IP.

```
curl -X POST http://<bot_ip>:3000/hello
curl -X POST http://<bot_ip>:3000/stretch
curl -X POST http://<bot_ip>:3000/content
curl -X POST http://<bot_ip>:3000/wallow
curl -X POST http://<bot_ip>:3000/dance1
curl -X POST http://<bot_ip>:3000/dance2
curl -X POST http://<bot_ip>:3000/pose
curl -X POST http://<bot_ip>:3000/scrape
```

---

## 🇧🇷 **Português — Definições das APIs**

| Constante                    | ID   | Endpoint   | Descrição da Ação                                                                               |
| ---------------------------- | ---- | ---------- | ----------------------------------------------------------------------------------------------- |
| `ROBOT_SPORT_API_ID_HELLO`   | 1016 | `/hello`   | O robô faz um gesto de “olá” — normalmente um aceno com a pata ou pequena reverência.           |
| `ROBOT_SPORT_API_ID_STRETCH` | 1017 | `/stretch` | O robô realiza um alongamento do corpo inteiro para preparar os motores.                        |
| `ROBOT_SPORT_API_ID_CONTENT` | 1020 | `/content` | O robô demonstra um comportamento de “feliz/contente”, com postura relaxada e animação de LEDs. |
| `ROBOT_SPORT_API_ID_WALLOW`  | 1021 | `/wallow`  | O robô rola ou balança de um lado para o outro de forma brincalhona.                            |
| `ROBOT_SPORT_API_ID_DANCE1`  | 1022 | `/dance1`  | O robô executa a rotina de dança #1 (mais rápida e energética).                                 |
| `ROBOT_SPORT_API_ID_DANCE2`  | 1023 | `/dance2`  | O robô executa a rotina de dança #2 (coreografia alternativa).                                  |
| `ROBOT_SPORT_API_ID_POSE`    | 1028 | `/pose`    | O robô entra em uma pose estática — ótimo para fotos e demonstrações.                           |
| `ROBOT_SPORT_API_ID_SCRAPE`  | 1029 | `/scrape`  | O robô faz o gesto de “raspar o chão” como um touro se preparando.                              |

---

### 🔧 **Exemplos em cURL**

```
curl -X POST http://<bot_ip>:3000/hello
curl -X POST http://<bot_ip>:3000/stretch
curl -X POST http://<bot_ip>:3000/content
curl -X POST http://<bot_ip>:3000/wallow
curl -X POST http://<bot_ip>:3000/dance1
curl -X POST http://<bot_ip>:3000/dance2
curl -X POST http://<bot_ip>:3000/pose
curl -X POST http://<bot_ip>:3000/scrape
```

## Notes

- The robot must be powered on and in SPORT mode.
- All actions are non-blocking: the robot will execute the motion sequence immediately.
