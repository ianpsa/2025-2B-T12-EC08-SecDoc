# 🔧 Troubleshooting

## Erro: "fatal error: 'stdbool.h' file not found" durante compilação

Este erro ocorre quando o `bindgen` (usado pela biblioteca `r2r`) não consegue encontrar os headers padrão do C. 

### Solução:

```bash
# 1. Instalar ferramentas de build essenciais e libclang
sudo apt update
sudo apt install build-essential clang libclang-dev

# 2. Garantir que o ROS2 está sourced
source /opt/ros/foxy/setup.bash  # Ajuste para sua versão (foxy, humble, etc)

# 3. Limpar cache e recompilar
cargo clean
cargo build --release
```

### Se ainda não funcionar:

```bash
# Instalar dependências adicionais do ROS2
sudo apt install \
    ros-foxy-rcl \
    ros-foxy-rcutils \
    ros-foxy-rmw \
    ros-foxy-rosidl-runtime-c

# Verificar versão do clang
clang --version

# Se necessário, instalar versão específica
sudo apt install clang-14 libclang-14-dev

# Exportar variável de ambiente para o bindgen
export LIBCLANG_PATH=/usr/lib/llvm-14/lib  # Ajuste conforme sua versão
export LLVM_CONFIG_PATH=/usr/lib/llvm-14/bin/llvm-config

# Tentar compilar novamente
cargo clean
cargo build --release
```

### Verificar se tudo está instalado:

```bash
# Verificar gcc
gcc --version

# Verificar clang
clang --version

# Verificar se stdbool.h existe
find /usr -name stdbool.h 2>/dev/null

# Verificar ROS2
echo $ROS_DISTRO
which ros2
```

## Erro: "undefined reference to rosidl_typesupport_introspection_c__get_message_type_support_handle__unitree_go"

Este erro ocorre quando o pacote ROS2 `unitree_go` não está corretamente compilado ou sourced antes de compilar o projeto Rust.

### Solução:

```bash
# 1. Source o ROS2 base
source /opt/ros/foxy/setup.bash

# 2. Source os workspaces que contêm os pacotes unitree_go
# (ajuste os caminhos conforme sua instalação)
source ~/go2_ros2_toolbox/install/setup.bash
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash

# 3. Verificar se os pacotes estão disponíveis
ros2 pkg list | grep unitree

# 4. Verificar se as mensagens existem
ros2 interface list | grep unitree_go

# 5. Recompilar o workspace unitree se necessário
cd ~/go2_ros2_toolbox
colcon build --packages-select unitree_go
source install/setup.bash

# 6. Limpar e recompilar o projeto Rust
cd /home/unitree/rust_serial_emergency_listener
cargo clean
cargo build --release
```

### Verificar conflitos entre workspaces

Se você tem múltiplos workspaces com o mesmo pacote (como mostrado no erro), pode haver conflitos:

```bash
# Verificar qual workspace está sendo usado
echo $AMENT_PREFIX_PATH

# Deve mostrar algo como:
# /home/unitree/go2_ros2_toolbox/install:/home/unitree/unitree_ros2/cyclonedds_ws/install:/opt/ros/foxy
```

### Recompilar pacotes unitree

Se as mensagens ainda não são encontradas:

```bash
# Ir para o workspace que contém unitree_go
cd ~/go2_ros2_toolbox  # ou ~/unitree_ros2/cyclonedds_ws

# Limpar e recompilar
colcon build --packages-select unitree_go --cmake-clean-cache

# Source novamente
source install/setup.bash

# Verificar se as mensagens foram geradas
ls install/unitree_go/lib/libunitree_go__rosidl_typesupport_c.so
ls install/unitree_go/lib/libunitree_go__rosidl_typesupport_introspection_c.so

# Agora compilar o projeto Rust
cd /home/unitree/rust_serial_emergency_listener
cargo clean
cargo build --release
```

### Script de build completo

Crie um script para facilitar:

```bash
#!/bin/bash
# build.sh

set -e  # Parar se houver erro

echo "🔧 Configurando ambiente ROS2..."
source /opt/ros/foxy/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash

echo "📦 Verificando pacotes ROS2..."
if ! ros2 pkg list | grep -q unitree_go; then
    echo "❌ Pacote unitree_go não encontrado!"
    exit 1
fi

echo "🧹 Limpando build anterior..."
cargo clean

echo "🔨 Compilando..."
cargo build --release

echo "✅ Compilação concluída!"
echo "📍 Binário: target/release/rust_serial_emergency_listener"
```

Torne-o executável e use:

```bash
chmod +x build.sh
./build.sh
```

### Atualizar install.sh

Se você tem um `install.sh`, garanta que ele source os workspaces corretos:

```bash
#!/bin/bash
source /opt/ros/foxy/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash

cargo build --release

# ... resto da instalação
```

## Erro: "unknown proxy name: 'cursor-bin'"

Este erro ocorre devido a uma configuração do rustup. Para corrigir:

```bash
# Remover override local do rustup
cd "/home/asvarius/Área de trabalho/rust_serial_emergency_listener"
rustup override unset

# Ou usar o toolchain padrão
rustup default stable
rustup update

# Tentar compilar novamente
cargo build --release
```

Se ainda não funcionar:

```bash
# Verificar toolchains instalados
rustup toolchain list

# Usar um toolchain específico
rustup default stable
cargo +stable build --release
```

## Erro: "Permission denied" ao abrir /dev/ttyACM0

```bash
# Adicionar ao grupo dialout
sudo usermod -a -G dialout $USER

# Verificar se foi adicionado
groups

# Se não aparecer "dialout", faça logout/login ou reinicie
```

Verificar permissões da porta:

```bash
ls -l /dev/ttyACM0
# Deve mostrar: crw-rw---- ... root dialout ...
```

## Porta serial não encontrada

### Descobrir qual porta usar:

```bash
# Método 1: Listar portas
ls -l /dev/tty{ACM,USB}*

# Método 2: Monitorar kernel logs
sudo dmesg -w
# Conecte o dispositivo USB e veja a saída

# Método 3: Usar udevadm
udevadm monitor --environment --udev
# Conecte o dispositivo
```

### Arduino não aparece:

```bash
# Verificar se foi detectado
lsusb

# Instalar drivers CH340/CH341 se necessário
sudo apt install ch341-driver

# Para Arduino genuíno
sudo apt install arduino
```

## ROS2 service call falha

### Verificar se ROS2 está configurado:

```bash
# Source do ROS2
source /opt/ros/humble/setup.bash  # ou sua versão

# Verificar variáveis de ambiente
env | grep ROS

# Deve ter:
# ROS_VERSION=2
# ROS_DISTRO=humble (ou outro)
```

### Verificar se o serviço existe:

```bash
# Listar todos os serviços
ros2 service list

# Procurar pelo serviço do Go2
ros2 service list | grep go2

# Ver tipo do serviço
ros2 service type /go2/modes
```

### Testar chamada manual:

```bash
ros2 service call /go2/modes go2_srvs/srv/Go2Modes "{request_data: damp}"
```

Se falhar, o problema é com o ROS2, não com este serviço.

### Instalar pacotes Go2:

```bash
# Se go2_srvs não está instalado
sudo apt update
sudo apt install ros-${ROS_DISTRO}-go2-srvs

# Ou compilar do source
cd ~/ros2_ws/src
git clone <repo_go2_srvs>
cd ~/ros2_ws
colcon build --packages-select go2_srvs
source install/setup.bash
```

## Serviço systemd não inicia

### Ver erros detalhados:

```bash
# Logs completos
sudo journalctl -u emergency-stop.service -n 100 --no-pager

# Últimas linhas
sudo journalctl -u emergency-stop.service -e

# Status detalhado
sudo systemctl status emergency-stop.service -l
```

### Problemas comuns:

#### 1. Executável não encontrado

```bash
# Verificar se existe
ls -la /home/asvarius/Área\ de\ trabalho/rust_serial_emergency_listener/target/release/rust_serial_emergency_listener

# Se não existir, compilar
cd "/home/asvarius/Área de trabalho/rust_serial_emergency_listener"
cargo build --release
```

#### 2. Permissões incorretas

```bash
# Garantir que o executável tem permissão de execução
chmod +x target/release/rust_serial_emergency_listener
```

#### 3. Caminho errado no .service

```bash
# Editar arquivo de serviço
sudo nano /etc/systemd/system/emergency-stop.service

# Verificar:
# - WorkingDirectory aponta para o diretório correto
# - ExecStart aponta para o executável correto
# - User existe no sistema

# Recarregar após editar
sudo systemctl daemon-reload
sudo systemctl restart emergency-stop.service
```

#### 4. Variáveis de ambiente ROS2

O serviço precisa das variáveis ROS2. Adicione ao arquivo .service:

```ini
[Service]
Environment="ROS_DOMAIN_ID=0"
Environment="RMW_IMPLEMENTATION=rmw_cyclonedds_cpp"

# Ou source do setup:
ExecStartPre=/bin/bash -c 'source /opt/ros/humble/setup.bash'
```

## Compilação falha

### Erro: "failed to resolve: use of undeclared crate or module"

```bash
# Atualizar dependências
cargo update

# Limpar cache e recompilar
cargo clean
cargo build --release
```

### Erro relacionado ao r2r

O r2r precisa do ROS2 instalado para compilar.

```bash
# Verificar se ROS2 está instalado
which ros2

# Source antes de compilar
source /opt/ros/humble/setup.bash
cargo build --release
```

### Erro: "linking with cc failed"

```bash
# Instalar dependências de build
sudo apt install build-essential pkg-config libssl-dev

# Para ROS2
sudo apt install ros-${ROS_DISTRO}-rcl ros-${ROS_DISTRO}-rcutils
```

## O botão não funciona

### 1. Verificar hardware

```bash
# Testar com monitor serial
# Arduino IDE: Tools > Serial Monitor
# Ou usar screen:
screen /dev/ttyACM0 115200

# Deve ver "0" e "1" quando pressiona o botão
# Pressione Ctrl+A depois K para sair
```

### 2. Verificar baud rate

Certifique-se que o baud_rate no `config/config.yaml` coincide com o do Arduino:

```yaml
baud_rate: 115200  # Deve ser igual ao Serial.begin() no Arduino
```

### 3. Testar leitura serial

```python
# Script Python simples
import serial
ser = serial.Serial('/dev/ttyACM0', 115200)
while True:
    line = ser.readline().decode().strip()
    print(f"Recebido: '{line}'")
```

## Performance

### Serviço usa muita CPU

Ajuste o delay no loop principal em `src/serial.rs`:

```rust
// Aumentar para reduzir uso de CPU
tokio::time::sleep(Duration::from_millis(50)).await;  // Era 10ms
```

Recompilar e reiniciar o serviço.

## Logs

### Aumentar verbosidade

Edite `src/main.rs`:

```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)  // Era INFO
    .init();
```

### Rotação de logs

O systemd já faz rotação automática. Para limitar:

```bash
# Editar configuração do journald
sudo nano /etc/systemd/journald.conf

# Adicionar:
SystemMaxUse=100M
SystemMaxFileSize=10M

# Reiniciar
sudo systemctl restart systemd-journald
```

## Ainda com problemas?

1. **Teste manual primeiro:** Execute `cargo run` e veja os erros detalhados
2. **Verifique os logs:** `sudo journalctl -u emergency-stop.service -f`
3. **Teste cada componente separadamente:**
   - Serial: Teste com script Python ou Arduino Serial Monitor
   - ROS2: Teste chamada manual do serviço
   - Integração: Execute `cargo run` manualmente

## Recursos Úteis

- ROS2 Docs: https://docs.ros.org/
- serialport-rs: https://docs.rs/serialport/
- r2r: https://github.com/sequenceplanner/r2r
- systemd: `man systemd.service`


