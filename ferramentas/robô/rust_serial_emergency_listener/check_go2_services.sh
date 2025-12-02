#!/bin/bash
# Script para descobrir os serviços e tópicos disponíveis do Go2

# Cores
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "🔍 Verificando serviços e tópicos do Go2 ROS2 Toolbox"
echo "====================================================="
echo ""

# Source do ambiente ROS2
if [ -f /opt/ros/foxy/setup.bash ]; then
    source /opt/ros/foxy/setup.bash
fi

if [ -f ~/go2_ros2_toolbox/install/setup.bash ]; then
    source ~/go2_ros2_toolbox/install/setup.bash
fi

# Verificar se ros2 está disponível
if ! command -v ros2 &> /dev/null; then
    echo "❌ ROS2 não encontrado. Source o ambiente primeiro:"
    echo "   source /opt/ros/foxy/setup.bash"
    echo "   source ~/go2_ros2_toolbox/install/setup.bash"
    exit 1
fi

echo -e "${BLUE}📦 Pacotes instalados relacionados ao Go2:${NC}"
ros2 pkg list | grep -E "(go2|unitree)" | sort
echo ""

echo -e "${BLUE}🔧 Serviços disponíveis (relacionados ao Go2):${NC}"
ros2 service list | grep -E "(go2|mode|emergency|stop)" | sort
if [ ${PIPESTATUS[1]} -ne 0 ]; then
    echo "  (Nenhum serviço Go2 encontrado - o robô pode não estar rodando)"
fi
echo ""

echo -e "${BLUE}📡 Tópicos disponíveis (relacionados ao Go2):${NC}"
ros2 topic list | grep -E "(go2|mode|emergency|stop|cmd)" | sort
if [ ${PIPESTATUS[1]} -ne 0 ]; then
    echo "  (Nenhum tópico Go2 encontrado - o robô pode não estar rodando)"
fi
echo ""

echo -e "${BLUE}📋 Tipos de serviço disponíveis (relacionados a modo/controle):${NC}"
ros2 interface list | grep -iE "(mode|control|command|stop)" | head -20
echo ""

echo -e "${YELLOW}💡 Dica:${NC}"
echo "Para descobrir o tipo de um serviço específico:"
echo "  ros2 service type /nome/do/servico"
echo ""
echo "Para ver a estrutura de um tipo de mensagem:"
echo "  ros2 interface show nome_do_pacote/srv/NomeDoServico"
echo ""
echo "Para ver todos os serviços ativos:"
echo "  ros2 service list"
echo ""

echo -e "${GREEN}✅ Verificação completa!${NC}"
echo ""
echo "Se você encontrou o serviço correto, atualize o arquivo:"
echo "  config/config.yaml"
echo ""
echo "E ajuste o código em:"
echo "  src/ros_client.rs (linha 38)"

