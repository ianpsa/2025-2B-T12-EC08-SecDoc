# Serviço de Botão de Emergência ROS2

Sistema em Rust para monitorar um botão de emergência via porta serial e acionar parada de emergência em robô Go2 via ROS2.

Este projeto usa o **go2_ros2_toolbox** para controlar o robô através do serviço `go2_srvs/srv/Go2Modes`.

## Pré-requisitos

### Sistema
- Ubuntu 20.04+ ou similar
- ROS2 (Humble, Iron, Jazzy ou foxy)
- Rust 1.70+ (instalar via [rustup](https://rustup.rs/))

### Hardware
- Dispositivo serial (ex: Arduino, ESP32) conectado via USB
- O dispositivo deve enviar "0" ou "1" seguido de newline (`\n`)

## Instalação

### ⭐ Método Recomendado para Go2 (go2_ros2_toolbox)

Se você está usando o **go2_ros2_toolbox**, use o script específico:

```bash
cd rust_serial_emergency_listener

# Copiar para a máquina do Go2 e executar:
./build_go2.sh
```

Este script:
- ✅ Verifica e compila o `go2_ros2_toolbox` se necessário
- ✅ Carrega todos os ambientes ROS2 corretamente
- ✅ Verifica pacotes `go2_srvs`, `unitree_go`, `unitree_api`
- ✅ Mostra diagnóstico de erros específicos
- ✅ Fornece próximos passos após compilação

📖 **Guia completo:** Consulte `BUILD_GO2.md` para instruções detalhadas.

### Método 2: Script Automático Completo

```bash
cd rust_serial_emergency_listener
./install.sh
```

O script irá:
- Instalar dependências do sistema
- Verificar Rust/Cargo
- Carregar ambiente ROS2
- Compilar o projeto
- Configurar o serviço systemd

### Método 3: Compilação Manual

#### 1. Instalar dependências

```bash
# Instalar ferramentas de build e clang (necessário para bindgen)
sudo apt update
sudo apt install build-essential clang libclang-dev pkg-config libudev-dev
```

#### 2. Carregar ambiente ROS2

```bash
# Source do ROS2 base
source /opt/ros/foxy/setup.bash

# Source dos workspaces unitree (ajuste os caminhos conforme sua instalação)
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash

# Verificar se os pacotes estão disponíveis
ros2 pkg list | grep unitree_go
```

#### 3. Compilar o projeto

```bash
# Usar o script de build simplificado
./build.sh

# OU compilar manualmente:
cargo clean
cargo build --release
```

### Método 4: Script de Build Simplificado

Se você encontrar problemas de compilação, use o script `build.sh`:

```bash
./build.sh
```

Este script:
- Verifica todas as dependências
- Carrega automaticamente os ambientes ROS2
- Mostra mensagens de erro detalhadas
- Fornece soluções para problemas comuns

## 📚 Guias e Documentação

Este projeto inclui vários guias para facilitar o uso:

- **`BUILD_GO2.md`** ⭐ - Guia específico para uso com go2_ros2_toolbox
- **`QUICK_FIX.md`** - Correções rápidas para erros comuns de compilação
- **`TROUBLESHOOTING.md`** - Soluções detalhadas para problemas
- **`README.md`** (este arquivo) - Documentação geral

### Scripts Disponíveis

- **`build_go2.sh`** ⭐ - Build otimizado para go2_ros2_toolbox
- **`build.sh`** - Build genérico com verificações
- **`install.sh`** - Instalação completa incluindo serviço systemd

### 3. Configurar a porta serial

Edite o arquivo `config/config.yaml` conforme necessário:

```yaml
serial_port: "/dev/ttyACM0"  # Ajuste para sua porta
baud_rate: 115200
ros_service_name: "/go2/modes"
ros_namespace: "emergency_stop"
```

Para descobrir sua porta serial:
```bash
ls /dev/tty*
# ou
dmesg | grep tty
```

### 4. Adicionar permissões de acesso à porta serial

```bash
# Adicionar seu usuário ao grupo dialout
sudo usermod -a -G dialout $USER

# Logout e login novamente para aplicar
# Ou reinicie o sistema
```

## Teste Manual

Antes de instalar como serviço, teste manualmente:

```bash
# Source do ROS2
source /opt/ros/foxy/setup.bash  # Ajuste conforme sua versão

# Ou, se compilou em release:
./rust_serial_emergency_listener
```

Teste enviando "1" pela serial para verificar se o serviço ROS2 é chamado.

## Instalar como Serviço Systemd

### 1. Copiar o arquivo de serviço

```bash
sudo cp systemd/emergency-stop.service /etc/systemd/system/
```

### 2. Editar o arquivo de serviço (se necessário)

```bash
sudo nano /etc/systemd/system/emergency-stop.service
```

**Ajustes importantes:**
- `User=` e `Group=` - seu usuário
- `WorkingDirectory=` - caminho completo do projeto
- `ExecStart=` - caminho do executável compilado
- Variáveis de ambiente ROS2 conforme sua configuração

### 3. Recarregar systemd e habilitar o serviço

```bash
# Recarregar configuração
sudo systemctl daemon-reload

# Habilitar para iniciar no boot
sudo systemctl enable emergency-stop.service

# Iniciar o serviço
sudo systemctl start emergency-stop.service
```

### 4. Verificar status

```bash
# Ver status
sudo systemctl status emergency-stop.service

# Ver logs em tempo real
sudo journalctl -u emergency-stop.service -f

# Ver logs das últimas horas
sudo journalctl -u emergency-stop.service --since "1 hour ago"
```

## Comandos Úteis

```bash
# Parar o serviço
sudo systemctl stop emergency-stop.service

# Reiniciar o serviço
sudo systemctl restart emergency-stop.service

# Desabilitar inicialização automática
sudo systemctl disable emergency-stop.service

# Ver logs com cores
sudo journalctl -u emergency-stop.service -f --output=cat
```

## Protocolo Serial

O dispositivo conectado à porta serial deve enviar:
- `1\n` - Botão de emergência PRESSIONADO (aciona parada)
- `0\n` - Botão de emergência LIBERADO (normal)

Qualquer outro valor será ignorado com um warning no log.

## Modificar Configuração

Para alterar a porta serial ou outros parâmetros **sem recompilar**:

```bash
# 1. Editar configuração
nano config/config.yaml

# 2. Reiniciar serviço
sudo systemctl restart emergency-stop.service
```

## 🐛 Troubleshooting

### Erro: "Permission denied" ao abrir porta serial

```bash
# Verificar se você está no grupo dialout
groups

# Se não estiver, adicionar:
sudo usermod -a -G dialout $USER
# Fazer logout/login ou reiniciar
```

### Serviço não inicia

```bash
# Ver logs detalhados
sudo journalctl -u emergency-stop.service -n 50 --no-pager

# Verificar se o executável existe
ls -la /home/asvarius/Área\ de\ trabalho/rust_serial_emergency_listener/target/release/rust_serial_emergency_listener

# Testar manualmente
cd /home/asvarius/Área\ de\ trabalho/rust_serial_emergency_listener
./target/release/rust_serial_emergency_listener
```

### Porta serial não encontrada

```bash
# Listar portas disponíveis
ls -la /dev/tty*

# Ver dispositivos USB conectados
lsusb

# Ver logs do kernel ao conectar dispositivo
sudo dmesg -w
# (conecte o dispositivo e veja qual porta foi atribuída)
```

### ROS2 service call falha

```bash
# Verificar se o ambiente go2_ros2_toolbox está sourced
source ~/go2_ros2_toolbox/install/setup.bash

# Verificar se o serviço ROS2 existe
ros2 service list | grep go2

# Testar chamada manual (coloca o robô em modo damping)
ros2 service call /go2/modes go2_srvs/srv/Go2Modes "{request_data: damp}"

# Verificar se o package go2_srvs está instalado
ros2 pkg list | grep go2

# Se go2_srvs não aparecer, recompilar
cd ~/go2_ros2_toolbox
colcon build --packages-select go2_srvs
source install/setup.bash
```

## Estrutura do Projeto

```
rust_serial_emergency_listener/
├── config/
│   └── config.yaml                 # Configuração (editável)
├── src/
│   ├── main.rs                     # Ponto de entrada
│   ├── config.rs                   # Gerenciamento de configuração
│   ├── serial.rs                   # Handler de porta serial
│   └── ros_client.rs               # Cliente ROS2
├── systemd/
│   └── emergency-stop.service      # Arquivo de serviço systemd
└── README.md                       # Este arquivo
└── Cargo.toml                      # Dependências Rust
```

## Segurança

**Importante:** Este sistema é crítico para segurança. Considere:

1. **Falhas de hardware:** O botão físico deve ser fail-safe
2. **Redundância:** Considere múltiplos métodos de parada de emergência
3. **Testes:** Teste regularmente o sistema completo
4. **Monitoramento:** Configure alertas para falhas do serviço
