#!/bin/bash
# Script de instalação rápida do serviço de emergência

set -e

echo "🚨 Instalação do Serviço de Botão de Emergência"
echo "================================================"
echo ""

# Cores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Verificar se está rodando como root
if [ "$EUID" -eq 0 ]; then 
    echo -e "${RED}⚠️  Don't run this script as root/sudo!${NC}"
    echo "Execute como usuário normal. O script pedirá sudo quando necessário."
    exit 1
fi

# 1. Verificar distribuição
echo "🔍 Verificando sistema operacional..."
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$NAME
    echo -e "${GREEN}✓${NC} Sistema: $OS"
else
    echo -e "${YELLOW}⚠️  Não foi possível detectar a distribuição${NC}"
    OS="Unknown"
fi
echo ""

# 2. Instalar dependências do sistema (Ubuntu/Debian)
echo "📦 Instalando dependências do sistema..."
if [[ "$OS" == *"Ubuntu"* ]] || [[ "$OS" == *"Debian"* ]]; then
    echo "Instalando pacotes necessários para Ubuntu/Debian..."
    sudo apt-get update
    sudo apt-get install -y \
        build-essential \
        pkg-config \
        libudev-dev \
        curl \
        git
    echo -e "${GREEN}✓${NC} Dependências do sistema instaladas"
else
    echo -e "${YELLOW}⚠️  Sistema não é Ubuntu/Debian. Certifique-se de ter instalado:${NC}"
    echo "  - build-essential (gcc, make, etc)"
    echo "  - pkg-config"
    echo "  - libudev-dev"
    read -p "Continuar? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi
echo ""

# 3. Instalar Rust se necessário
echo "🦀 Verificando instalação do Rust..."
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}⚠️  Rust/Cargo não encontrado. Instalando...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}✓${NC} Rust instalado com sucesso"
else
    echo -e "${GREEN}✓${NC} Rust já está instalado"
fi

# Garantir que cargo está no PATH
if ! command -v cargo &> /dev/null; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

echo -e "${GREEN}✓${NC} Rust/Cargo: $(cargo --version)"
echo ""

# 4. Carregar ambiente ROS2
echo "🤖 Carregando ambiente ROS2..."
ROS_LOADED=false

# Tentar carregar ROS2 Foxy
if [ -f /opt/ros/foxy/setup.bash ]; then
    echo "  Carregando /opt/ros/foxy/setup.bash..."
    source /opt/ros/foxy/setup.bash
    ROS_LOADED=true
fi

# Tentar carregar unitree_ros2
if [ -f "$HOME/unitree_ros2/setup.sh" ]; then
    echo "  Carregando ~/unitree_ros2/setup.sh..."
    source "$HOME/unitree_ros2/setup.sh"
elif [ -f "unitree_ros2/setup.sh" ]; then
    echo "  Carregando ./unitree_ros2/setup.sh..."
    source unitree_ros2/setup.sh
fi

# Tentar carregar go2_ros2_toolbox
if [ -f "$HOME/go2_ros2_toolbox/install/setup.bash" ]; then
    echo "  Carregando ~/go2_ros2_toolbox/install/setup.bash..."
    cd "$HOME/go2_ros2_toolbox" && source install/setup.bash
    cd - > /dev/null
elif [ -f "go2_ros2_toolbox/install/setup.bash" ]; then
    echo "  Carregando ./go2_ros2_toolbox/install/setup.bash..."
    cd go2_ros2_toolbox && source install/setup.bash
    cd - > /dev/null
fi

if [ "$ROS_LOADED" = true ] && command -v ros2 &> /dev/null; then
    echo -e "${GREEN}✓${NC} Ambiente ROS2 carregado: ROS_DISTRO=$ROS_DISTRO"
else
    echo -e "${YELLOW}⚠️  ROS2 não encontrado ou não carregado completamente${NC}"
    echo "Certifique-se de ter ROS2 instalado. O serviço continuará sem ROS2 no PATH."
    read -p "Continuar mesmo assim? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi
echo ""

# 5. Compilar o projeto
echo "🔨 Compilando o projeto..."
CURRENT_DIR=$(pwd)

echo "  Executando: cargo build --release"
cargo build --release

if [ ! -f "target/release/rust_serial_emergency_listener" ]; then
    echo -e "${RED}❌ Falha na compilação!${NC}"
    echo "O binário não foi gerado em target/release/"
    exit 1
fi

echo -e "${GREEN}✓${NC} Compilação concluída com sucesso"
echo ""

# 6. Copiar binário para o diretório raiz do projeto
echo "📋 Copiando binário..."
cp target/release/rust_serial_emergency_listener ./
chmod +x rust_serial_emergency_listener
echo -e "${GREEN}✓${NC} Binário copiado para: $CURRENT_DIR/rust_serial_emergency_listener"
echo ""

# 7. Verificar grupo dialout
echo "🔐 Verificando permissões de porta serial..."

if ! groups | grep -q dialout; then
    echo -e "${YELLOW}⚠️  Você não está no grupo 'dialout'${NC}"
    echo "Adicionando ao grupo..."
    sudo usermod -a -G dialout $USER
    echo -e "${GREEN}✓${NC} Adicionado ao grupo dialout"
    echo -e "${YELLOW}⚠️  Você precisa fazer logout/login ou reiniciar para aplicar${NC}"
else
    echo -e "${GREEN}✓${NC} Permissões OK"
fi
echo ""

# 8. Configurar arquivo de configuração
echo "⚙️  Configuração..."

if [ ! -f config/config.yaml ]; then
    echo -e "${RED}❌ Arquivo config/config.yaml não encontrado!${NC}"
    exit 1
fi

# Detectar porta serial automaticamente
echo "Detectando portas seriais disponíveis:"
ls /dev/ttyACM* /dev/ttyUSB* 2>/dev/null || echo "  Nenhuma porta detectada"
echo ""

read -p "Porta serial (padrão: /dev/ttyACM0): " SERIAL_PORT
SERIAL_PORT=${SERIAL_PORT:-/dev/ttyACM0}

# Atualizar config.yaml
sed -i "s|serial_port:.*|serial_port: \"$SERIAL_PORT\"|" config/config.yaml

echo -e "${GREEN}✓${NC} Configuração atualizada: $SERIAL_PORT"
echo ""

# 9. Instalar serviço systemd
echo "🔧 Instalando serviço systemd..."

# Atualizar caminhos no arquivo de serviço
CURRENT_USER=$(whoami)

# Criar arquivo de serviço com paths corretos e ambiente ROS2
cat > /tmp/emergency-stop.service << EOF
[Unit]
Description=Emergency Stop Service for Robot
After=network.target

[Service]
Type=simple
User=$CURRENT_USER
Group=$CURRENT_USER
WorkingDirectory=$CURRENT_DIR
ExecStart=/bin/bash -c 'source /opt/ros/foxy/setup.bash && source \$HOME/unitree_ros2/setup.sh && cd \$HOME/go2_ros2_toolbox && source install/setup.bash && cd $CURRENT_DIR && $CURRENT_DIR/rust_serial_emergency_listener'
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo cp /tmp/emergency-stop.service /etc/systemd/system/
sudo systemctl daemon-reload

echo -e "${GREEN}✓${NC} Serviço instalado"
echo ""

# 10. Perguntar se quer habilitar
read -p "Habilitar serviço para iniciar no boot? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    sudo systemctl enable emergency-stop.service
    echo -e "${GREEN}✓${NC} Serviço habilitado"
fi
echo ""

# 11. Perguntar se quer iniciar agora
read -p "Iniciar serviço agora? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    sudo systemctl start emergency-stop.service
    sleep 2
    echo ""
    echo "Status do serviço:"
    sudo systemctl status emergency-stop.service --no-pager
fi

echo ""
echo "================================================"
echo -e "${GREEN}✅ Instalação Concluída! ${NC}"
echo "================================================"
echo ""
echo "📋 Comandos úteis:"
echo "  sudo systemctl status emergency-stop.service   # Ver status"
echo "  sudo journalctl -u emergency-stop.service -f   # Ver logs em tempo real"
echo "  sudo systemctl restart emergency-stop.service  # Reiniciar"
echo "  sudo systemctl stop emergency-stop.service     # Parar"
echo ""
echo "📝 Arquivos:"
echo "  Binário: $CURRENT_DIR/rust_serial_emergency_listener"
echo "  Configuração: $CURRENT_DIR/config/config.yaml"
echo "  Documentação: $CURRENT_DIR/README.md"
echo ""


