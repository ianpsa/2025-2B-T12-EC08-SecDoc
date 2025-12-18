# Rust Audio Player - High Performance

Player de áudio otimizado para baixa latência, projetado para streaming de áudio via WebSocket para o robô Unitree GO2.

## Arquitetura

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  WebSocket  │───▶│ Worker Pool │───▶│ Ring Buffer │───▶│   Robot     │
│  Receiver   │    │ (4 FFmpeg)  │    │  (Ordered)  │    │  Megaphone  │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

### Otimizações Implementadas

1. **Worker Pool Paralelo**: 4 workers FFmpeg em threads nativas processando chunks simultaneamente
2. **Reordenação Automática**: BTreeMap garante playback na ordem correta mesmo com decode paralelo
3. **Ring Buffer**: VecDeque para inserção/remoção O(1) vs O(n) do Vec
4. **Channels Zero-Copy**: crossbeam-channel para comunicação de alta performance
5. **Parking Lot Mutexes**: Mais rápidos que std::sync::Mutex
6. **Pre-buffering**: Acumula N chunks antes de iniciar playback (reduz stuttering)
7. **Fire-and-Forget**: Não espera confirmação de playback

## Uso

```bash
# Compilar (release para máxima performance)
cargo build --release

# Executar
./target/release/rust_audio_player [robot_ip] [websocket_url]

# Exemplo
./target/release/rust_audio_player 192.168.123.161 ws://localhost:8765
```

## Servidor de Teste

```bash
# Na pasta pai
python test_websocket_client.py song.mp3
```

## Configuração

| Parâmetro | Default | Descrição |
|-----------|---------|-----------|
| PRE_BUFFER_COUNT | 2 | Chunks a bufferar antes de iniciar |
| MAX_BUFFER | 16 | Tamanho máximo do buffer |
| WORKER_COUNT | 4 | Workers FFmpeg paralelos |
| CHUNK_MS | 200 | Duração do chunk (servidor) |

## Performance

- **Latência típica**: ~400ms (2 chunks x 200ms)
- **Throughput**: ~50 chunks/segundo
- **Memória**: ~20MB (buffer + temp files)

## Dependências

- FFmpeg (para decode)
- Python 3.8+ (para play_audio.py)
- Conexão com robô Unitree GO2
