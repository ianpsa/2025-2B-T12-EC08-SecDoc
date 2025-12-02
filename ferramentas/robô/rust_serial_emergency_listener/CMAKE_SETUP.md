# 🔧 Configuração do CMAKE_PREFIX_PATH para Compilação

## O Problema

O erro `undefined reference to rosidl_typesupport_introspection_c__get_message_type_support_handle__unitree_go` acontece quando o linker não encontra as bibliotecas do `unitree_go`.

O `r2r` (biblioteca Rust para ROS2) usa as variáveis de ambiente para encontrar as bibliotecas:
- `CMAKE_PREFIX_PATH` - Onde procurar bibliotecas
- `AMENT_PREFIX_PATH` - Workspaces ROS2
- `LD_LIBRARY_PATH` - Bibliotecas dinâmicas

## ✅ Solução: Source Correto dos Ambientes

### Passo 1: Verificar o Estado Atual

```bash
# Verificar se as variáveis estão definidas
echo "CMAKE_PREFIX_PATH: $CMAKE_PREFIX_PATH"
echo "AMENT_PREFIX_PATH: $AMENT_PREFIX_PATH"
echo "ROS_DISTRO: $ROS_DISTRO"
```

**Saída esperada:**
```
CMAKE_PREFIX_PATH: /home/unitree/go2_ros2_toolbox/install:/home/unitree/unitree_ros2/cyclonedds_ws/install:/opt/ros/foxy
AMENT_PREFIX_PATH: /home/unitree/go2_ros2_toolbox/install:/home/unitree/unitree_ros2/cyclonedds_ws/install:/opt/ros/foxy
ROS_DISTRO: foxy
```

### Passo 2: Source dos Ambientes na Ordem Correta

**IMPORTANTE:** A ordem importa!

```bash
# 1. ROS2 base (sempre primeiro)
source /opt/ros/foxy/setup.bash

# 2. Workspaces adicionais (na ordem de dependência)
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash

# 3. Verificar novamente
echo $CMAKE_PREFIX_PATH
```

### Passo 3: Verificar se as Bibliotecas Existem

```bash
# Procurar pela biblioteca unitree_go
find ~/go2_ros2_toolbox/install -name "*unitree_go*rosidl*.so" 2>/dev/null
find ~/unitree_ros2/cyclonedds_ws/install -name "*unitree_go*rosidl*.so" 2>/dev/null
```

**Deve encontrar algo como:**
```
.../install/unitree_go/lib/libunitree_go__rosidl_typesupport_c.so
.../install/unitree_go/lib/libunitree_go__rosidl_typesupport_introspection_c.so
.../install/unitree_go/lib/libunitree_go__rosidl_generator_c.so
```

Se **NÃO** encontrar, significa que o pacote `unitree_go` não foi compilado corretamente.

## 🔍 Diagnóstico de Problemas

### Problema 1: CMAKE_PREFIX_PATH vazio

**Sintoma:**
```bash
echo $CMAKE_PREFIX_PATH
# (sem saída)
```

**Solução:**
```bash
# Source manual
source /opt/ros/foxy/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash
```

### Problema 2: Bibliotecas unitree_go não encontradas

**Sintoma:**
```bash
find ~/go2_ros2_toolbox -name "*unitree_go*.so"
# (sem resultados)
```

**Solução:** O workspace não tem o pacote compilado. Verifique se está no workspace certo:

```bash
# Verificar qual workspace tem unitree_go
ls ~/go2_ros2_toolbox/install/
ls ~/unitree_ros2/cyclonedds_ws/install/

# Se unitree_go estiver em cyclonedds_ws:
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash

# Verificar novamente
echo $CMAKE_PREFIX_PATH | grep cyclonedds
```

### Problema 3: Ordem errada de source

Se você fizer source na ordem errada, os paths podem ficar incorretos.

**Solução:** Sempre source na ordem:
1. ROS2 base
2. Workspaces de dependências
3. Seu workspace

## 🚀 Script de Build com Verificação Automática

O `build_simple.sh` agora verifica automaticamente:

```bash
./build_simple.sh
```

Ele mostrará:
- ✅ CMAKE_PREFIX_PATH configurado
- ✅ Bibliotecas unitree_go encontradas
- ❌ Problemas detectados com instruções

## 📝 Compilação Manual com Variáveis Corretas

Se preferir compilar manualmente:

```bash
# 1. Source dos ambientes
source /opt/ros/foxy/setup.bash
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash

# 2. Verificar
env | grep -E "(CMAKE_PREFIX|AMENT_PREFIX|ROS_DISTRO)"

# 3. Compilar
cargo clean
cargo build --release
```

## 🔗 Adicionar ao .bashrc (Opcional)

Para não precisar source toda vez:

```bash
# Editar .bashrc
nano ~/.bashrc

# Adicionar no final:
source /opt/ros/foxy/setup.bash
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash

# Recarregar
source ~/.bashrc
```

**⚠️ Cuidado:** Isso afeta TODOS os terminais. Só faça se usar ROS2 constantemente.

## 🧪 Teste Rápido

Script para testar se tudo está correto:

```bash
#!/bin/bash
# test_cmake_paths.sh

echo "🔍 Testando configuração CMAKE..."
echo ""

# Source
source /opt/ros/foxy/setup.bash
source ~/go2_ros2_toolbox/install/setup.bash 2>/dev/null
source ~/unitree_ros2/cyclonedds_ws/install/setup.bash 2>/dev/null

# Verificar CMAKE_PREFIX_PATH
if [ -z "$CMAKE_PREFIX_PATH" ]; then
    echo "❌ CMAKE_PREFIX_PATH não definido!"
    exit 1
fi

echo "✅ CMAKE_PREFIX_PATH definido:"
echo "$CMAKE_PREFIX_PATH" | tr ':' '\n' | sed 's/^/   - /'
echo ""

# Verificar unitree_go
echo "🔍 Procurando bibliotecas unitree_go..."
FOUND=false
IFS=':' read -ra PATHS <<< "$CMAKE_PREFIX_PATH"
for path in "${PATHS[@]}"; do
    if [ -f "$path/lib/libunitree_go__rosidl_typesupport_c.so" ]; then
        echo "✅ Encontrado em: $path/lib/"
        FOUND=true
    fi
done

if [ "$FOUND" = false ]; then
    echo "❌ Bibliotecas unitree_go não encontradas!"
    exit 1
fi

echo ""
echo "✅ Tudo OK! Pode compilar:"
echo "   cargo build --release"
```

Salve como `test_cmake_paths.sh`, torne executável e rode:

```bash
chmod +x test_cmake_paths.sh
./test_cmake_paths.sh
```

## 📚 Resumo

**Para compilar com sucesso, você PRECISA:**

1. ✅ Source do ROS2 base
2. ✅ Source dos workspaces com unitree_go
3. ✅ CMAKE_PREFIX_PATH apontando para os installs corretos
4. ✅ Bibliotecas .so do unitree_go existindo

**Comando completo:**

```bash
source /opt/ros/foxy/setup.bash && \
source ~/go2_ros2_toolbox/install/setup.bash && \
cargo clean && \
cargo build --release
```

Ou simplesmente:

```bash
./build_simple.sh
```

O script agora verifica tudo automaticamente! 🎯

