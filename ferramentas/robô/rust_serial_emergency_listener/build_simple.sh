#!/bin/bash
# build_simple.sh - Build simples que NÃO mexe em nada do ROS2
# Apenas compila o projeto Rust usando o que já está instalado

set -e

# Cores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "🔨 Build Simples - Rust Emergency Listener"
echo "=========================================="
echo ""
echo "ℹ️  Este script NÃO recompila pacotes ROS2"
echo "   Usa apenas o que já está instalado"
echo ""

# 1. Instalar Rust se necessário
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}⚠️  Rust não encontrado. Instalando...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "  ${GREEN}✓${NC} Rust instalado"
fi

# 2. Instalar dependências de sistema se necessário
echo "📦 Verificando dependências..."
MISSING=()

if ! command -v gcc &> /dev/null; then
    MISSING+=("build-essential")
fi

if ! command -v clang &> /dev/null; then
    MISSING+=("clang libclang-dev")
fi

if [ ${#MISSING[@]} -ne 0 ]; then
    echo -e "${YELLOW}Instalando dependências de sistema...${NC}"
    sudo apt update
    sudo apt install -y build-essential clang libclang-dev pkg-config libudev-dev
fi

echo -e "  ${GREEN}✓${NC} Dependências OK"
echo ""

# 3. Source do ambiente ROS2 (sem recompilar nada)
echo "📦 Carregando ambiente ROS2..."

# ROS2 base
if [ -f /opt/ros/foxy/setup.bash ]; then
    source /opt/ros/foxy/setup.bash
    echo -e "  ${GREEN}✓${NC} ROS2 Foxy"
fi

# go2_ros2_toolbox (se existir)
if [ -f ~/go2_ros2_toolbox/install/setup.bash ]; then
    source ~/go2_ros2_toolbox/install/setup.bash
    echo -e "  ${GREEN}✓${NC} go2_ros2_toolbox"
fi

# cyclonedds_ws (se existir)
if [ -f ~/unitree_ros2/cyclonedds_ws/install/setup.bash ]; then
    source ~/unitree_ros2/cyclonedds_ws/install/setup.bash
    echo -e "  ${GREEN}✓${NC} cyclonedds_ws"
fi

echo ""

# 4. Verificar variáveis de ambiente ROS2
echo "🔍 Verificando variáveis de ambiente..."
echo "   CMAKE_PREFIX_PATH: ${CMAKE_PREFIX_PATH:-(não definido)}"
echo "   AMENT_PREFIX_PATH: ${AMENT_PREFIX_PATH:-(não definido)}"
echo "   ROS_DISTRO: ${ROS_DISTRO:-(não definido)}"

if [ -z "$CMAKE_PREFIX_PATH" ]; then
    echo -e "${RED}❌ CMAKE_PREFIX_PATH não está definido!${NC}"
    echo "Certifique-se de que o ambiente ROS2 foi sourced:"
    echo "   source /opt/ros/foxy/setup.bash"
    echo "   source ~/go2_ros2_toolbox/install/setup.bash"
    exit 1
fi

echo -e "  ${GREEN}✓${NC} Variáveis de ambiente OK"
echo ""

# 5. Verificar se bibliotecas unitree_go existem
echo "📚 Verificando bibliotecas unitree_go..."
UNITREE_LIB_FOUND=false

# Procurar em todos os paths do CMAKE_PREFIX_PATH
IFS=':' read -ra PATHS <<< "$CMAKE_PREFIX_PATH"
for path in "${PATHS[@]}"; do
    if [ -f "$path/lib/libunitree_go__rosidl_typesupport_c.so" ]; then
        echo -e "  ${GREEN}✓${NC} Encontrado em: $path/lib/"
        UNITREE_LIB_FOUND=true
        break
    fi
done

if [ "$UNITREE_LIB_FOUND" = false ]; then
    echo -e "  ${YELLOW}⚠️  Biblioteca unitree_go não encontrada nos paths conhecidos${NC}"
    echo "  Isso pode causar erro de linking."
    echo ""
    echo "  Caminhos verificados:"
    for path in "${PATHS[@]}"; do
        echo "    - $path/lib/"
    done
fi

echo ""

# 6. Compilar
echo "⚙️  Compilando projeto Rust..."
echo ""

cargo clean
if cargo build --release; then
    echo ""
    echo -e "${GREEN}✅ Compilação bem-sucedida!${NC}"
    echo ""
    
    if [ -f "target/release/rust_serial_emergency_listener" ]; then
        cp target/release/rust_serial_emergency_listener ./
        chmod +x rust_serial_emergency_listener
        
        echo "📦 Binário pronto:"
        echo "   $(pwd)/rust_serial_emergency_listener"
        echo ""
        echo "🚀 Para testar:"
        echo "   source /opt/ros/foxy/setup.bash"
        echo "   source ~/go2_ros2_toolbox/install/setup.bash"
        echo "   ./rust_serial_emergency_listener"
    fi
else
    echo ""
    echo -e "${RED}❌ Falha na compilação${NC}"
    echo ""
    echo "Consulte ALTERNATIVAS_SEM_GO2_SRVS.md para ajustar o código"
    exit 1
fi

