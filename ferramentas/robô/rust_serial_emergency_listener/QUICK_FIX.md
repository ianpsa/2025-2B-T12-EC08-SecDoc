# 🚀 Correção Rápida - Erros de Compilação

## ⚡ Solução Ultra-Rápida para go2_ros2_toolbox

Se você está usando **go2_ros2_toolbox**, use o script otimizado:

```bash
cd ~/rust_serial_emergency_listener
./build_go2.sh
```

Este script faz tudo automaticamente! ✨

---

## Para o erro atual: "undefined reference to rosidl_typesupport_introspection_c__get_message_type_support_handle__unitree_go"

Execute estes comandos **na máquina onde está compilando** (`/home/unitree/rust_serial_emergency_listener`):

### Passo 1: Instalar dependências de sistema

```bash
sudo apt update
sudo apt install -y build-essential clang libclang-dev pkg-config libudev-dev
```

### Passo 2: Carregar ambiente ROS2 e go2_ros2_toolbox

```bash
# Source do ROS2
source /opt/ros/foxy/setup.bash

# Source do go2_ros2_toolbox (IMPORTANTE!)
source ~/go2_ros2_toolbox/install/setup.bash

# Source do cyclonedds_ws se existir (opcional mas recomendado)
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash
```

### Passo 3: Verificar se os pacotes necessários estão disponíveis

```bash
# Verificar pacotes unitree
ros2 pkg list | grep unitree

# Verificar pacotes go2
ros2 pkg list | grep go2
```

**Saída esperada:**
```
go2_core
go2_description
go2_srvs          ← IMPORTANTE para o serviço de emergência
unitree_api       ← IMPORTANTE
unitree_go        ← IMPORTANTE
unitree_hg
```

Se **NÃO** aparecer `unitree_go` ou `go2_srvs`, você precisa compilá-los:

```bash
# Compilar o go2_ros2_toolbox completo
cd ~/go2_ros2_toolbox
colcon build --cmake-clean-cache
source install/setup.bash

# OU compilar apenas os pacotes necessários
cd ~/go2_ros2_toolbox
colcon build --packages-select unitree_go go2_srvs --cmake-clean-cache
source install/setup.bash

# Verificar novamente
ros2 pkg list | grep -E "(unitree|go2)"
```

### Passo 4: Compilar o projeto Rust

```bash
cd ~/rust_serial_emergency_listener

# Limpar builds anteriores
cargo clean

# Compilar
cargo build --release
```

---

## Scripts Automatizados

Para facilitar, você pode usar um dos scripts de build:

### Para go2_ros2_toolbox (Recomendado):

```bash
cd ~/rust_serial_emergency_listener
./build_go2.sh
```

### Build genérico:

```bash
cd ~/rust_serial_emergency_listener
./build.sh
```

---

## Comandos Completos em Sequência (go2_ros2_toolbox)

Copie e cole tudo de uma vez:

```bash
# Instalar dependências
sudo apt update && sudo apt install -y build-essential clang libclang-dev pkg-config libudev-dev

# Carregar ambiente ROS2 e go2_ros2_toolbox
source /opt/ros/foxy/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash

# Opcional: cyclonedds_ws
if [ -f ~/unitree_ros2/cyclonedds_ws/install/setup.bash ]; then
    source ~/unitree_ros2/cyclonedds_ws/install/setup.bash
fi

# Verificar pacotes necessários
echo "Verificando pacotes..."
ros2 pkg list | grep -E "(unitree|go2)"

# Se go2_srvs ou unitree_go não aparecerem, compilá-los
if ! ros2 pkg list | grep -q "go2_srvs\|unitree_go"; then
    echo "Compilando go2_ros2_toolbox..."
    cd ~/go2_ros2_toolbox
    colcon build --cmake-clean-cache
    source install/setup.bash
fi

# Verificar interface Go2Modes
ros2 interface show go2_srvs/srv/Go2Modes

# Compilar o projeto Rust
cd ~/rust_serial_emergency_listener
cargo clean
cargo build --release

# Se compilou com sucesso, o binário estará em:
# ~/rust_serial_emergency_listener/target/release/rust_serial_emergency_listener
```

---

## Solução de Problemas Específicos

### Se ainda der erro "stdbool.h not found"

```bash
sudo apt install clang libclang-dev

# Verificar instalação
clang --version
find /usr -name stdbool.h 2>/dev/null
```

### Se dar erro "undefined reference to rosidl_typesupport..."

Significa que os workspaces ROS2 não foram sourced corretamente. Certifique-se de:

1. Source do ROS2 base: `source /opt/ros/foxy/setup.bash`
2. Source dos workspaces com unitree_go
3. Verificar com `echo $AMENT_PREFIX_PATH` que os caminhos estão corretos

### Se der erro "linking with cc failed"

```bash
# Instalar bibliotecas ROS2 necessárias
sudo apt install \
    ros-foxy-rcl \
    ros-foxy-rcutils \
    ros-foxy-rmw \
    ros-foxy-rosidl-runtime-c
```

---

## Criar Script de Build Permanente

Crie um arquivo `~/rust_serial_emergency_listener/build_local.sh`:

```bash
#!/bin/bash
set -e

echo "🔧 Carregando ambiente ROS2..."
source /opt/ros/foxy/setup.bash
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash

echo "🔨 Compilando..."
cargo clean
cargo build --release

echo "✅ Pronto!"
ls -lh target/release/rust_serial_emergency_listener
```

Torne executável:
```bash
chmod +x ~/rust_serial_emergency_listener/build_local.sh
```

Agora sempre que precisar recompilar:
```bash
cd ~/rust_serial_emergency_listener
./build_local.sh
```

---

## Testar o Binário

Após compilar com sucesso:

```bash
# Source ROS2 novamente (se em novo terminal)
source /opt/ros/foxy/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash

# Executar
cd ~/rust_serial_emergency_listener
./target/release/rust_serial_emergency_listener
```

---

## Dúvidas?

Consulte:
- `README.md` - Documentação completa
- `TROUBLESHOOTING.md` - Solução de problemas detalhada
- Logs de erro em `/tmp/cargo_build.log` (ao usar build.sh)

