#!/bin/bash
# Script de instalação do Emergency Stop Service como daemon systemd
# 
# Uso: sudo ./install_daemon.sh

set -e  # Para na primeira falha

# Cores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Emergency Stop Service - Instalador de Daemon ===${NC}\n"

# Verifica se está rodando como root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}❌ Este script precisa ser executado como root!${NC}"
    echo -e "   Use: ${YELLOW}sudo ./install_daemon.sh${NC}"
    exit 1
fi

# Variáveis de configuração
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="/opt/emergency-stop"
BIN_NAME="rust_serial_emergency_listener"
SERVICE_NAME="emergency-stop.service"
CONFIG_DIR="/opt/emergency-stop/config"

echo -e "${YELLOW}📁 Diretório do projeto: ${PROJECT_DIR}${NC}"
echo -e "${YELLOW}📦 Instalação em: ${INSTALL_DIR}${NC}\n"

# PASSO 1: Compilar em modo release
echo -e "${GREEN}[1/6] Compilando em modo release...${NC}"
cd "$PROJECT_DIR"

# Verifica se ROS2 está configurado
if [ -z "$ROS_DISTRO" ]; then
    echo -e "${YELLOW}⚠️  ROS_DISTRO não configurado. Tentando carregar ambiente ROS2...${NC}"
    
    # Tenta carregar o setup.sh do unitree_ros2
    UNITREE_ROS2_DIR="$HOME/unitree_ros2"
    if [ -f "$UNITREE_ROS2_DIR/setup.sh" ]; then
        echo -e "${YELLOW}   Carregando ambiente do Unitree ROS2...${NC}"
        cd "$UNITREE_ROS2_DIR"
        source setup.sh
        cd "$PROJECT_DIR"
    elif [ -f "/opt/ros/foxy/setup.bash" ]; then
        echo -e "${YELLOW}   Carregando ROS2 Foxy...${NC}"
        source /opt/ros/foxy/setup.bash
        if [ -f "$HOME/unitree_ros2/cyclonedds_ws/install/setup.bash" ]; then
            source $HOME/unitree_ros2/cyclonedds_ws/install/setup.bash
        fi
        export RMW_IMPLEMENTATION=rmw_cyclonedds_cpp
        export ROS_DOMAIN_ID=0
    else
        echo -e "${RED}❌ ROS2 não encontrado! Configure manualmente.${NC}"
        exit 1
    fi
fi

echo "   Executando: cargo build --release"
cargo build --release

if [ ! -f "target/release/$BIN_NAME" ]; then
    echo -e "${RED}❌ Falha na compilação! Binário não encontrado.${NC}"
    exit 1
fi
echo -e "${GREEN}   ✓ Compilação concluída${NC}\n"

# PASSO 2: Criar estrutura de diretórios
echo -e "${GREEN}[2/6] Criando estrutura de diretórios...${NC}"
mkdir -p "$INSTALL_DIR/bin"
mkdir -p "$CONFIG_DIR"
echo -e "${GREEN}   ✓ Diretórios criados${NC}\n"

# PASSO 3: Copiar binário
echo -e "${GREEN}[3/6] Instalando binário...${NC}"
cp "target/release/$BIN_NAME" "$INSTALL_DIR/bin/"
chmod +x "$INSTALL_DIR/bin/$BIN_NAME"
echo -e "${GREEN}   ✓ Binário instalado em: $INSTALL_DIR/bin/$BIN_NAME${NC}\n"

# PASSO 4: Copiar arquivo de configuração
echo -e "${GREEN}[4/6] Copiando configurações...${NC}"
if [ -f "config/config.yaml" ]; then
    cp "config/config.yaml" "$CONFIG_DIR/"
    echo -e "${GREEN}   ✓ config.yaml copiado${NC}"
else
    echo -e "${YELLOW}   ⚠️  config/config.yaml não encontrado, pulando...${NC}"
fi
echo ""

# PASSO 5: Configurar permissões
echo -e "${GREEN}[5/6] Configurando permissões...${NC}"
CURRENT_USER=$(logname || echo $SUDO_USER)
chown -R $CURRENT_USER:$CURRENT_USER "$INSTALL_DIR"

# Adiciona usuário ao grupo dialout (acesso à porta serial)
usermod -a -G dialout $CURRENT_USER || echo "   ⚠️  Não foi possível adicionar ao grupo dialout"
echo -e "${GREEN}   ✓ Permissões configuradas para usuário: $CURRENT_USER${NC}\n"

# PASSO 6: Instalar serviço systemd
echo -e "${GREEN}[6/6] Instalando serviço systemd...${NC}"
if [ -f "systemd/$SERVICE_NAME" ]; then
    cp "systemd/$SERVICE_NAME" "/etc/systemd/system/"
    systemctl daemon-reload
    echo -e "${GREEN}   ✓ Serviço instalado em: /etc/systemd/system/$SERVICE_NAME${NC}"
else
    echo -e "${RED}   ❌ Arquivo systemd/$SERVICE_NAME não encontrado!${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ Instalação concluída com sucesso!${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════${NC}\n"

echo -e "${YELLOW}📋 Próximos passos:${NC}\n"
echo -e "   1. Habilitar o serviço (iniciar automaticamente no boot):"
echo -e "      ${GREEN}sudo systemctl enable $SERVICE_NAME${NC}\n"

echo -e "   2. Iniciar o serviço agora:"
echo -e "      ${GREEN}sudo systemctl start $SERVICE_NAME${NC}\n"

echo -e "   3. Verificar status:"
echo -e "      ${GREEN}sudo systemctl status $SERVICE_NAME${NC}\n"

echo -e "   4. Ver logs em tempo real:"
echo -e "      ${GREEN}sudo journalctl -u $SERVICE_NAME -f${NC}\n"

echo -e "   5. Parar o serviço:"
echo -e "      ${GREEN}sudo systemctl stop $SERVICE_NAME${NC}\n"

echo -e "   6. Desabilitar inicialização automática:"
echo -e "      ${GREEN}sudo systemctl disable $SERVICE_NAME${NC}\n"

echo -e "${YELLOW}⚠️  IMPORTANTE:${NC}"
echo -e "   - O usuário '$CURRENT_USER' foi adicionado ao grupo 'dialout'"
echo -e "   - Faça logout/login para as permissões terem efeito"
echo -e "   - Ajuste o arquivo /opt/emergency-stop/config/config.yaml se necessário\n"

exit 0
