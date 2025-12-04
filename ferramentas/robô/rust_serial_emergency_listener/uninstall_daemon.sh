#!/bin/bash
# Script de desinstalação do Emergency Stop Service
# 
# Uso: sudo ./uninstall_daemon.sh

set -e

# Cores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}=== Emergency Stop Service - Desinstalador ===${NC}\n"

# Verifica root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}❌ Este script precisa ser executado como root!${NC}"
    echo -e "   Use: ${YELLOW}sudo ./uninstall_daemon.sh${NC}"
    exit 1
fi

SERVICE_NAME="emergency-stop.service"
INSTALL_DIR="/opt/emergency-stop"

echo -e "${YELLOW}⚠️  Isso irá remover completamente o serviço.${NC}"
read -p "   Deseja continuar? (s/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Ss]$ ]]; then
    echo -e "${GREEN}Cancelado.${NC}"
    exit 0
fi

echo ""
echo -e "${GREEN}[1/4] Parando serviço...${NC}"
systemctl stop $SERVICE_NAME 2>/dev/null || echo "   Serviço já estava parado"
echo ""

echo -e "${GREEN}[2/4] Desabilitando serviço...${NC}"
systemctl disable $SERVICE_NAME 2>/dev/null || echo "   Serviço já estava desabilitado"
echo ""

echo -e "${GREEN}[3/4] Removendo arquivos do sistema...${NC}"
rm -f "/etc/systemd/system/$SERVICE_NAME"
systemctl daemon-reload
echo -e "${GREEN}   ✓ Arquivo de serviço removido${NC}"

if [ -d "$INSTALL_DIR" ]; then
    rm -rf "$INSTALL_DIR"
    echo -e "${GREEN}   ✓ Diretório $INSTALL_DIR removido${NC}"
fi
echo ""

echo -e "${GREEN}[4/4] Limpando cache...${NC}"
systemctl reset-failed 2>/dev/null || true
echo ""

echo -e "${GREEN}════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ Desinstalação concluída!${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════${NC}\n"

exit 0
