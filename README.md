# Organização Security/Safety - Equipe Sec Robô

> **Dica**: Termos técnicos? Consulte o [Glossário](#glossário) no final deste documento.

## Índice

1. [Premissas](#premissas)
2. [Modelo Operacional (SSDLC)](#modelo-operacional-security-by-design--ssdlc)
3. [Governança e RACI](#governança-e-raci)
4. [SIEM por Analista (Owners por Domínio)](#siem-por-analista-owners-por-domínio)
   - [Esquema de Log Canônico](#esquema-de-log-canônico-comum-a-todos)
   - [Analista 3 – DevOps/UX](#analista-3--devopsux-siem-ux)
   - [Analista 2 – Backend](#analista-2--backend-siem-be)
   - [Analista 1 – Modelo/LLM](#analista-1--modelollm-siem-llm)
   - [Analista 4 – Robô](#analista-4--robô-siem-rob)
5. [Grupo Interno: CTI/Enablement](#grupo-interno-ctienablement-misp-treinamento-estratégia-e-guidelines)
6. [Controles e Gates (GitHub + Actions)](#controles-e-gates-github--actions)
7. [SIEM e Observabilidade](#siem-e-observabilidade-mínimo-viável)
8. [Integrações MISP ↔ SIEM](#integrações-misp--siem)
9. [Mapeamento NIST CSF](#mapeamento-nist-csf)
10. [Cobertura RF/RNF](#cobertura-rfrnf-essencial)
11. [KPIs/KRIs](#kpiskris)
12. [Playbooks](#playbooks)
13. [Roteiro de 10 semanas](#roteiro-de-10-semanas-com-siem-por-analista-e-misp)
14. [Entregáveis](#entregáveis)
15. [Tarefas detalhadas por equipe](#tarefas-detalhadas-por-equipe)
    - [Grupo 1 — Analistas de Segurança Integrados](#grupo-1--analistas-de-segurança-integrados)
    - [Grupo 2 — Security Testing & Tooling](#grupo-2--security-testing--tooling)
    - [Tarefas Transversais](#tarefas-transversais-analistas-14--blue-team)

## Premissas

- Plataforma de versionamento/CI: GitHub + Actions
- SIEM inexistente: será implantado (recomendado: Wazuh + OpenSearch)
- Gestão de segredos: GitHub Secrets/Actions OIDC + .env em runtime, nunca hard-coded
- Se houver transcrição de áudio, preferir WhisperX na pipeline de PLN

## Modelo Operacional (Security by Design / SSDLC)

- Descoberta e requisitos: threat modeling leve por feature (STRIDE), requisitos de segurança/safety em cada User Story
- Design: padrões de arquitetura seguros (JWT/MFA, rate limiting, DTLS/WebRTC, logs estruturados, RAG seguro)
- Implementação: padrões de coding seguros + pre-commit hooks (lint/tests/secret-scan)
- Verificação (CI): gates obrigatórios em PR (SAST, SCA, Secrets, IaC, testes, cobertura mínima, image scan)
- Deploy: ambientes efêmeros por PR; DAST/ZAP; aprovação dupla; SBOM publicado
- Operação: observability (métricas, logs, traces), SIEM, alertas < 60 s, playbooks IR/DR, CTI via MISP
- Melhoria contínua: red-team validation, post-mortems, hardening iterativo

## Governança e RACI

### O que é RACI?

**RACI** é uma matriz:
- **R** = Responsible (Responsável): quem executa a tarefa
- **A** = Accountable (Aprovador): responsável final e decisor
- **C** = Consulted (Consultado): quem dá input profissional
- **I** = Informed (Informado): quem é comunicado do progresso

---

### Organização:

- DevOps/UX: <br>
**R=Analista 3 (Owner SIEM do domínio), A=Scrum, C=Blue Team, I=Red Team**
- Backend: <br>
**R=Analista 2 (Owner SIEM do domínio), A=Scrum, C=Blue Team, I=Red Team**
- Modelo (LLM): <br> **R=Analista 1 (Owner SIEM do domínio), A=Scrum, C=Blue Team, I=Red Team**
- Robô: <br> **R=Analista 4 (Owner SIEM do domínio), A=Scrum, C=Blue Team, I=Red Team**
- CTI/Enablement (MISP, Treinamento, Estratégia, Guidelines): <br> **R=Grupo 2 (Blue Team), A=Scrum, C=Analistas 1-4, I=Red Team**

## SIEM por Analista (Owners por Domínio)

Cada Analista Integrado implementará e manterá a instrumentação SIEM na arquitetura do seu domínio, garantindo coleta, normalização, dashboards e alertas alinhados a RF/RNF.

### Esquema de Log Canônico (comum a todos)

- Campos base: timestamp, trace_id, span_id, correlation_id, env, asset_id, component, service.name, user_id/session_id (quando aplicável)
- Evento: event.category (auth, control, estop, model, network, iac, ci), event.type (start, success, failure, anomaly), severity, message
- Métricas: latency_ms, rtt_ms, packet_loss_pct, cpu_mem_pct, queue_size, rate_limit_hits, ack_stop_ms
- Privacidade: mascarar PII (ocultar dados pessoais nos logs); retenção ≥ 90 dias; at-rest (quando os dados já estão no disco) AES-256; in-transit (quando os dados forem transmitidos) TLS

### Analista 3 – DevOps/UX (SIEM-UX)

- Coletas: OTel JS (OpenTelemetry) no front; logs de Nginx/CDN; eventos de UI (E-Stop, filtros, buscas); erros de UX; Lighthouse budgets (ferramenta do google para auditar qualidade de sites)
- Dashboards: performance p95 (<1s operações críticas), acessibilidade, taxa de erros, funil de formulários (definido pela Ana Garcia)
- Alertas: STOP acionado sem ACK em 1s; degradação de tempo de resposta; secret leak detectado em PR

### Analista 2 – Backend (SIEM-BE)

- Coletas: logs JSON (request/response), validação JWT/MFA, RBAC (Atributos de decisãi de controle de acesso), rate limiter (100 req/min/IP), DB audit, fila/mensageria
- Dashboards: latência p95 por endpoint (robô ≤200ms, LLM ≤500ms), erros 5xx, violações de rate limit, throughput
- Alertas: auth falha em massa, anomalia de payload, queda no ACK STOP, CVE crítico em lib usada

### Analista 1 – Modelo/LLM (SIEM-LLM)

- Coletas: decisões do Modelo A (classe, severidade, motivo), bloqueios A→B, prompts sanitizados, fontes RAG, latências
- Dashboards: taxa de bloqueio, falsos positivos (<2%), latência total ≤1,5s, qualidade/ética ≥95%
- Alertas: deriva/enchimento do contexto, falhas do modelo menor, aumento de recusas injustificadas pelo modelo A

### Analista 4 – Robô (SIEM-ROB)

- Coletas: telemetria (bateria, IMU, motores), eventos E-Stop, failsafe, WebRTC (RTT/bitrate/perdas), ROS 2 logs, sensores (LiDAR/proximidade)
- Dashboards: uptime ≥99%, reconexão <3s, colisões=0, limites torque/velocidade
- Alertas: perda de conexão, obstáculos não tratados, falhas de sensor, STOP

## Grupo Interno: CTI/Enablement (MISP, Treinamento, Estratégia e Guidelines)

- MISP (CTI): provisionar MISP, ingestão de feeds (OSINT, ISACs), curadoria de IoCs, taxonomias; integração com SIEM para regras/alertas
- Ferramentas de instrução: portal de conhecimento, runbooks e playbooks, simulações (tabletop e drills), e-learning para operadores
- Estratégia: Security by Design por feature; critérios de severidade (CIS 16.6); matriz de risco; revisão trimestral
- Guidelines do “cachorro robô” (safety):
- Operação: perímetro seguro, velocidade/torque máximos, política de aproximação, fallback manual, politica de desenvolvimento (interno)
- Privacidade: política de captura de áudio/vídeo; consentimento e sinalização
- Ética/Conteúdo: proibições, linguagem adequada, respostas verificáveis, bloqueio de termos
- Incidentes: E-Stop, comunicação e reporte

## Controles e Gates (GitHub + Actions)

- Proteção de branch/main; 2 revisores; status checks required
- SAST: CodeQL; Semgrep/Bandit conforme linguagem
- SCA: Dependabot + Grype/OWASP DC; bloqueio CVE crítico
- Secrets: GitHub secret scanning + Gitleaks (pre-commit/CI)
- IaC: Checkov/tfsec; OPA/Conftest
- Build: SBOM (Syft); Trivy; assinatura (cosign) opcional
- Testes: unit/integration; cobertura mínima 80%; carga (E-Stop/latência)
- DAST: OWASP ZAP em env efêmero

## SIEM e Observabilidade (mínimo viável)

- Stack: Wazuh Manager + OpenSearch; OTel Collector; Filebeat/Fluent Bit; agentes Wazuh
- Retenção: 90 dias; criptografia at-rest/in-transit
- Casos de uso comuns: auth/JWT, rate limit, E-Stop, falhas do robô, auditoria LLM
- Alertas SLA: ≤ 60 s; ChatOps/Email

## Integrações MISP ↔ SIEM

- Pull de IoCs MISP → regras Wazuh/OSSigma → alertas SIEM
- Correlação: eventos locais + IoCs (IPs/domínios/hashes) → casos de incidente
- Governança: curadoria semanal; expiração de IoCs; avaliação de eficácia

## Mapeamento NIST CSF

- Identify: inventário/classificação; risk register por sprint
- Protect: MFA, gestão de segredos, criptografia, gates de CI/CD
- Detect: SIEM central, detecção de anomalias por domínio
- Respond: playbooks (E-Stop/IR), comunicação e mitigação
- Recover: backups/restauração, DR test e melhoria

## Cobertura RF/RNF (essencial)

- E-Stop fim-a-fim com ACK ≤1s; logs/auditoria
- JWT/MFA; rate limiting; logs criptografados (retenção ≥90d)
- LLM com A→B, auditoria e ética; latência ≤1,5s
- Robô com DTLS/WebRTC, reconexão <3s, QoS crítico e limites físicos

## KPIs/KRIs

- KPIs: % PRs aprovados nos gates; p95 latências; ACK E-Stop; cobertura testes; MTTR vulnerabilidades
- KRIs: segredos em PR; violações rate limit; reconexões falhas; respostas LLM inadequadas

## Playbooks

- E-Stop: botão→ACK→corte→logs→notificação→post-mortem
- IR: classificação, contenção, erradicação, lições aprendidas
- DR: restore de artefatos/configs, retomada segura

## Roteiro de 10 semanas (com SIEM por analista e MISP)

### Sprint 2 (Semanas 3–4)
- Instrumentar UX e Backend (mínimo); Playbooks E-Stop/IR
- Provisionar MISP e feeds principais; guia inicial do robô (safety)
- Branch protection/2FA; CodeQL/SCA/Secrets; Trivy/SBOM
- Definir esquema canônico de logs; PoC SIEM (Wazuh/OpenSearch)


### Sprint 3 (Semanas 5–6)
- Instrumentar Modelo e Robô; OTel completo; MFA painel; painel de rate limit

### Sprint 4 (Semanas 7–8)
- Integração MISP↔SIEM (regras/alertas); treinamentos e drills
- DTLS/WebRTC robusto; QoS ROS 2; auditoria LLM (Modelo A)

### Sprint 5 (Semanas 9–10)
- Pentest full-scope; exercícios tabletop; DR test
- Dashboards executivos; KPIs/KRIs; hardening de pipelines
- Revisão de guidelines do robô e CTI

## Entregáveis

<details>
<summary>Políticas: acesso, segredos, logging/auditoria, IR/DR, CTI (MISP): </summary>
<br>

#### Exemplo:

- IR-001 — E-Stop indevido
    - Responsável: Analista 4
    - SLA de resposta: 5 min
    - Passos: isolar → logar → reiniciar → mitigar → documentar
- IR-002 — Falha de Autenticação
    - Responsável: Analista 2
    - SLA de resposta: 5 min
    - Passos: isolar → rever logs → restaurar certificado → validar → documentar
- IR-003 — LLM não responde
    - Responsável: Analista 1
    - SLA de resposta: 2 min
    - Passos: isolar → logar → fallback → restart → escalar → documentar
- DR-001 — Falha total de infra
    - Responsável: Coordenação
    - RTO: 15 min, RPO: 1 min
    - Falha simula: incêndio no datacenter
- DR-002 — Banco de dados perdido
    - Responsável: Analista 2 + DBA
    - RTO: 30 min, RPO: 5 min
    - Falha simula: DB corrompido

</details>

> esse lance aqui encima é um dropdown viu?

- Pipelines GitHub Actions com gates e relatórios
- SIEM operacional por domínio + painéis executivos
- MISP com feeds/curadoria + integração com SIEM
- Guia de uso seguro do robô + trilha de treinamento
- Matriz RF/RNF ↔ controles; Relatório de pentest e plano de correção

## Tarefas detalhadas por equipe

### Grupo 1 — Analistas de Segurança Integrados

#### Analista 1 — Modelo/LLM (Owner SIEM-LLM)
- Definir taxonomia de eventos LLM (A→B, classe, severidade, motivo, latências)
- Instrumentar logs canônicos no Modelo A (detector) e Modelo B (respondente)
- Encaminhar logs via OTel/Fluent Bit para SIEM e normalizar no pipeline
- Criar dashboards (taxa de bloqueio, FP/FN, p95/p99 latência, uso de RAG)
- Configurar alertas (p95 > 1,5 s; falha do A; aumento de recusas injustificadas; erro canal interno)
- Implementar gating rígido A→B e filtros de conteúdo (listas de bloqueio/permitidos)
- Sanitizar prompts/contexto (RAG) e mascarar PII nos logs
- Tentativas de ataque OWASP LLM Top 10 (prompt injection, data exfiltration, poisoning)
- Registrar auditoria completa das decisões do A (entrada sanitizada, saída, justificativa)
- Garantir mTLS/ACLs entre serviços de inferência e proteção de segredos (se aplicável no WebRTC)
- Estabelecer rotina de auditoria mensal e testes de reprodutibilidade dos modelos

#### Analista 2 — Backend (Owner SIEM-BE)
- Instrumentar logs estruturados com trace_id/correlation_id (OTel + JSON)
- Registrar validações de JWT/MFA, RBAC/ABAC (role based acess control/Attribute based access control ex: time, ip, device, location, ...) e eventos de rate limiting

```
RBAC controla “o que você pode fazer” e ABAC considera “quando, onde e de onde você está fazendo”
```

- Ativar auditoria de banco (DDL/DML sensível) e de filas/mensageria

```
DDL (Data Definition Language): alterações de schema; auditoria deve capturar quem executou (user_principal) e quando (timestamp), com a query/campo afetado.
DML (Data Modification Language): mudanças de dados; entidades/atributos afetados, valores antigos e novos (quando aplicável) e resultado da operação.
```

- Criar dashboards (p95 robô ≤ 200 ms; p95 LLM ≤ 500 ms; 5xx; violações rate limit)
- Configurar alertas (falhas de auth em massa; queda do ACK STOP; payloads anômalos)
- Implementar JWT correto (iss/aud/exp curto), rotação de chaves e MFA no painel

```
tokens do jwt que indicam quem emitiu, para quem e validade
```

- Aplicar rate limiting adaptativo com painel de ajuste e logs de violações
- Validar inputs (schema, tamanho, anti-XXE/SQLi/NoSQLi) e respostas (headers seguros)
- Integrar CodeQL/Semgrep, Dependabot/Grype, Gitleaks, Checkov/tfsec, Trivy/Syft
- Bloquear PR/Deploy por CVE crítico e exigir cobertura mínima de testes (≥ 80%)

#### Analista 3 — DevOps/UX (Owner SIEM-UX)
- Instrumentar front-end com OTel JS e coletar logs de Nginx/CDN e erros de UI
- Registrar eventos críticos de UI (E-Stop, filtros, buscas, falhas de formulário)
- Criar dashboards (p95 < 1 s operações críticas; acessibilidade; taxa de erros; funil)
- Configurar alertas (E-Stop sem ACK ≤ 1 s; degradação p95; segredo detectado em PR)
- Aplicar proteção de branch, 2 revisores, 2FA e secret scanning na organização
- Implementar mecanismos anti-bot e registrar tentativas bloqueadas
- Automatizar testes de acessibilidade e budgets de performance em CI
- Garantir UX safety (E-Stop visível, estados de erro, modos degradados/offline)

#### Analista 4 — Robô (Owner SIEM-ROB)
- Coletar telemetria (bateria/IMU), ROS 2 logs (se aplicável), métricas WebRTC (RTT/perdas/bitrate)
- Logar E-Stop, failsafe, reconexão e verificação de origem/integridade de comandos
- Criar dashboards (uptime ≥ 99%; reconexão < 3 s; colisões = 0; limites torque/velocidade)
- Criar redundâncias de segurança quanto à movimentação do robô
- Configurar alertas (perda de conexão; falhas de sensor; STOP sem corte de potência)
- Implementar DTLS/WebRTC seguro (certificados, identidade, priorização de mensagens)
- Ajustar QoS ROS 2 para canais críticos e watchdog de hardware
- Validar limites físicos (torque/velocidade), zonas seguras e plano de fallback
- Executar testes de falha induzida (rede/sensor) e medir STOP fim-a-fim ≤ 1 s

### Grupo 2 — Security Testing & Tooling

#### Red Team — Pentests e Ferramentas de Ataque
- Definir escopo/regras e cenários de ameaça (API/Web/Robô/LLM):
    - Basear-se no OWASP top 10
- Executar pentests (OWASP API/Web; WebRTC/DTLS; ROS 2; OWASP LLM)
- Tentar bypass de rate limit, JWT attacks, MITM, fuzzing de comandos do robô
- Construir PoCs de exploração e priorizar achados por severidade/impacto
- Registrar achados no SIEM/MISP e realizar retestes após correções
- Conduzir sessões purple team com Blue Team e Analistas 1–4

#### Blue Team — Proteção, CTI (MISP) e Enablement
- Provisionar MISP, integrar feeds (OSINT/ISACs) e definir taxonomias
- Integrar MISP↔SIEM (IoCs→regras/alertas) e curadoria semanal de IoCs
- Provisionar Wazuh+OpenSearch e criar regras Sigma e correlações por domínio
- Montar painéis executivos e runbooks de investigação/contensão
- Construir modelo menor validador (guard rails) e integrar telemetria ao SIEM
- Elaborar guidelines do robô (safety, ética, privacidade, incidentes) e treinar equipe
- Definir KPIs/KRIs e conduzir cadência mensal de revisão no comitê de segurança

### Tarefas Transversais (Analistas 1–4 + Blue Team)
- Unificar esquema canônico de logs e sincronização de tempo (NTP)
- Padronizar instrumentação OTel e correlação trace_id/correlation_id entre domínios (grupos de analistas)
- Garantir gestão de segredos (GitHub Secrets/OIDC) e proibir segredos em código
- Estabelecer matriz de rastreabilidade RF/RNF ↔ controles ↔ alertas SIEM
- Manter e testar playbooks (E-Stop, IR, DR, failsafe) e registrar lições aprendidas

```
IR -> incident response; DR -> Disaster Recovery
```

- Publicar SBOM por release e aplicar gates de SAST/SCA/Secrets/IaC/DAST no CI/CD
- Configurar SLA de alertas ≤ 60 s e rotas de notificação (ChatOps/Email)
- Revisar trimestralmente estratégia de segurança, riscos e guidelines do robô

<br>

# Glossário

### Índice Rápido
**A**: [ABAC](#abac-attribute-based-access-control), [ACK](#ack-acknowledgment), [ACLs](#acls-access-control-lists), [AES-256](#aes-256-advanced-encryption-standard)

**B**: [Blue Team](#blue-team)

**C**: [CIS 16.6](#cis-166), [CTI](#cti-cyber-threat-intelligence), [CVE](#cve-common-vulnerabilities-and-exposures)

**D**: [DAST](#dast-dynamic-application-security-testing), [DDL](#ddl-data-definition-language), [DML](#dml-data-modification-language), [DR](#dr-disaster-recovery), [DTLS](#dtls-datagram-transport-layer-security)

**E**: [E-Stop](#e-stop-emergency-stop)

**I**: [IoC](#ioc-indicator-of-compromise), [IR](#ir-incident-response), [ISAC](#isac-information-sharing-and-analysis-center)

**J**: [JWT](#jwt-json-web-token)

**K**: [KPIs](#kpis-key-performance-indicators), [KRIs](#kris-key-risk-indicators)

**L**: [LLM](#llm-large-language-model)

**M**: [MFA](#mfa-multi-factor-authentication), [MISP](#misp-malware-information-sharing-platform), [mTLS](#mtls-mutual-tls)

**N**: [NIST CSF](#nist-csf-national-institute-of-standards-and-technology-cybersecurity-framework)

**O**: [OSINT](#osint-open-source-intelligence), [OSSigma](#ossigma), [OTel](#otel-opentelemetry), [OWASP](#owasp-open-web-application-security-project)

**P**: [PII](#pii-personally-identifiable-information), [Purple Team](#purple-team)

**R**: [RACI](#raci), [RAG](#rag-retrieval-augmented-generation), [RBAC](#rbac-role-based-access-control), [Red Team](#red-team), [RF/RNF](#rfrnf-requisitos-funcionaisrequisitos-não-funcionais), [ROS 2](#ros-2-robot-operating-system), [RPO](#rpo-recovery-point-objective), [RTO](#rto-recovery-time-objective)

**S**: [SAST](#sast-static-application-security-testing), [SBOM](#sbom-software-bill-of-materials), [SCA](#sca-software-composition-analysis), [SIEM](#siem-security-information-and-event-management), [SSDLC](#ssdlc-secure-software-development-life-cycle), [STRIDE](#stride)

**T**: [TLS](#tls-transport-layer-security)

**W**: [Wazuh](#wazuh), [WebRTC](#webrtc-web-real-time-communication), [WhisperX](#whisperx)

---

### ABAC (Attribute-Based Access Control)
Controle de acesso baseado em atributos. Considera "quando, onde e de onde você está fazendo". Exemplos: horário (time), localização (location), dispositivo (device), IP.

### ACK (Acknowledgment)
Confirmação de recebimento. No contexto de E-Stop, é a confirmação que o sistema recebeu o comando de parada de emergência.

### AES-256 (Advanced Encryption Standard)
Algoritmo de criptografia simétrica com chave de 256 bits. Usado para criptografar dados em repouso (at-rest).

### Blue Team
Equipe de segurança focada em defesa e proteção. É consultada (C) nos projetos RACI.

### CVE (Common Vulnerabilities and Exposures)
Identificador público para vulnerabilidades de segurança. Exemplo: "CVE-2024-12345".

### DAST (Dynamic Application Security Testing)
Teste de segurança dinâmico que verifica aplicações em execução. Exemplo: OWASP ZAP.

### DDL (Data Definition Language)
Linguagem de definição de dados. Estruturas de banco: CREATE, ALTER, DROP. É auditado para capturar alterações de schema.

### DML (Data Modification Language)
Linguagem de manipulação de dados. Operações: INSERT, UPDATE, DELETE. É auditado para capturar mudanças em dados sensíveis.

### DTLS (Datagram Transport Layer Security)
Protocolo de segurança para conexões UDP. Usado em comunicação com robô via WebRTC.

### E-Stop (Emergency Stop)
Botão de parada de emergência do robô. Deve responder com ACK ≤ 1s.

### IoC (Indicator of Compromise)
Sinal de atividade maliciosa. Inclui IPs, domínios, hashes. Curado no MISP e integrado ao SIEM.

### IR (Incident Response)
Resposta a incidentes de segurança. Processo que inclui classificação, contenção e erradicação.

### JWT (JSON Web Token)
Token de autenticação em JSON. Contém claims como "iss" (issuer), "aud" (audience), "exp" (expiration).

### KPIs (Key Performance Indicators)
Métricas de desempenho. Exemplos: % PRs aprovados, p95 de latências, cobertura de testes ≥80%.

### KRIs (Key Risk Indicators)
Indicadores de risco de segurança. Exemplos: segredos em PR, respostas LLM inadequadas.

### LLM (Large Language Model)
Modelo de linguagem grande, como GPT ou modelos customizados. Pode ter modelo A (detector) e modelo B (respondente).

### MFA (Multi-Factor Authentication)
Autenticação multi-fator. Adiciona camada de segurança além de senha (ex: SMS, app autenticador).

### MISP (Malware Information Sharing Platform)
Plataforma de compartilhamento de informações sobre ameaças (CTI). Usado para IoCs, feeds OSINT/ISACs.

### NIST CSF (National Institute of Standards and Technology Cybersecurity Framework)
Framework de segurança cibernética. 5 funções: Identify, Protect, Detect, Respond, Recover.

### OTel (OpenTelemetry)
Padrão aberto para observabilidade (métricas, logs, traces). OTel JS é usado no frontend.

### OWASP (Open Web Application Security Project)
Projeto de segurança web. Mantém listas como OWASP Top 10 (Web, API, LLM).

### PII (Personally Identifiable Information)
Informação pessoalmente identificável. Deve ser mascarada nos logs (ex: CPF, email).

### Purple Team
Colaboração entre Red Team e Blue Team. Sessões conjuntas para aprimorar defesas.

### RACI
Matriz de responsabilidades: Responsible (R), Accountable (A), Consulted (C), Informed (I).

### RAG (Retrieval-Augmented Generation)
Geração aumentada por recuperação. LLM usa banco de dados para respostas mais precisas.

### RBAC (Role-Based Access Control)
Controle de acesso baseado em papéis. Define "o que você pode fazer" por função.

### Red Team
Equipe de segurança focada em testes ofensivos. Executa pentests e é informada (I) no RACI.

### ROS 2 (Robot Operating System)
Sistema operacional para robôs. Logs ROS 2 são coletados pelo Analista 4.

### SAST (Static Application Security Testing)
Teste de segurança estático. Analisa código-fonte sem executar. Exemplo: CodeQL, Semgrep.

### SCA (Software Composition Analysis)
Análise de composição de software. Detecta vulnerabilidades em dependências. Exemplo: Dependabot, Grype.

### SBOM (Software Bill of Materials)
Lista de componentes de software. Publicado por release usando Syft.

### SIEM (Security Information and Event Management)
Sistema de gerenciamento de eventos de segurança. No plano: Wazuh + OpenSearch.

### STRIDE
Framework de threat modeling. 6 categorias: Spoofing, Tampering, Repudiation, Information disclosure, Denial of service, Elevation of privilege.

### TLS (Transport Layer Security)
Protocolo de criptografia para transporte. HTTPS usa TLS para proteger dados in-transit.

### WebRTC (Web Real-Time Communication)
Protocolo de comunicação em tempo real. Usado para comunicação com robô via DTLS.

### WhisperX
Framework para transcrição de áudio. Preferido sobre Whisper para maior precisão.

### ACLs (Access Control Lists)
Listas de controle de acesso. Definem permissões de acesso a recursos. Usado entre serviços de inferência.

### CIS 16.6
Critérios de severidade do Center for Internet Security. Usado para avaliar vulnerabilidades.

### CTI (Cyber Threat Intelligence)
Inteligência de ameaças cibernéticas. Informações sobre ataques, táticas, técnicas e procedimentos.

### DR (Disaster Recovery)
Recuperação de desastres. Plano para restaurar sistemas após falhas críticas. RTO/RPO são métricas importantes.

### ISAC (Information Sharing and Analysis Center)
Centro de compartilhamento e análise de informações. Compartilha IoCs entre organizações.

### mTLS (Mutual TLS)
TLS mutuo. Autenticação bidirecional usando certificados. Mais seguro que TLS normal.

### OSSigma
Formato de regras de detecção aberto. Compatível com SIEMs para criar alertas padronizados.

### OSINT (Open Source Intelligence)
Inteligência de fontes abertas. Informações públicas usadas para identificar ameaças.

### RTO (Recovery Time Objective)
Tempo objetivo de recuperação. Tempo máximo aceitável para restauração após desastre.

### RPO (Recovery Point Objective)
Objetivo de ponto de recuperação. Máximo de dados que podem ser perdidos (ex: último backup).

### RF/RNF (Requisitos Funcionais/Requisitos Não-Funcionais)
Requisitos funcionais (o que o sistema faz) e não-funcionais (performance, segurança). Mapeados para controles.

### SSDLC (Secure Software Development Life Cycle)
Ciclo de vida de desenvolvimento de software seguro. Integra segurança em todas as fases.

### Wazuh
Solução SIEM open-source. Detecta intrusões, vulnerabilidades e anomalias de segurança.
