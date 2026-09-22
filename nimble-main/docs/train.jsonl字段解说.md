# train.jsonl 字段完全解说

> 数据位置：`nimble-main/data/train.jsonl`（2,676 行，每行一条独立 JSON）+ `eval.jsonl`（324 行，结构相同）。
> 一句话：每条数据 = 一道"只许从给定选项里选一个"的判断题，外加一整套"这道题出得合不合格"的质检档案。

## 一、顶层字段（13 个）

| 字段 | 类型 | 含义 |
|------|------|------|
| `domain` | str | 题目所属行业。共 10 类：commerce（电商）、education（教育）、home（家庭）、media（媒体）、public_services（公共服务）、science（科学）、software（软件）、supply_chain（供应链）、travel（旅行）、workplace（职场） |
| `family` | str | 所属"题族"编号。同一个源头造出来的所有题（含它的反事实孪生题）共享一个族号，防止训练/考试数据泄漏时同族题一半在练习册一半在考卷 |
| `id` | str | 本条数据唯一编号，如 `scale-diverse-013-002-base`（末尾 base/counterfactual 表明它是原版还是改 fact 版） |
| `input` | obj | **题干本体**：问题 + 证据材料（详见下） |
| `method` | str | 造法标记。全部 2,676 条都是 `c2d`（contrastive data curation，对比式造数），说明这批数据是流水线统一出品的 |
| `provenance` | obj | 来源溯源：源头数据集编号、是否合成（全是 true）、内容指纹 sha256、属于 train 还是 eval 切分 |
| `quality_status` | str | 质检等级。全部为 `evidence_model_checked` = "由模型审核过"（注意：没有任何一条经过人工审核） |
| `reference` | obj | **标准答案**及它的产生方式（详见下） |
| `source_family` | str | 更一层的家族归组（如 `commerce-03`），用于切分统计 |
| `split` | str | 归属：`train`（练习册）或 `eval`（考卷）。本文件里全是 train |
| `variant` | str | 孪生标记：`base`（原版，1,338 条）或 `counterfactual`（改掉一个关键事实的版本，1,338 条）。两者配成对，正确答案必然不同——这就是"对比学习"的载体 |

## 二、`input` —— 题干

```
input
├── questions
│   └── decision          本题唯一的问题（每条数据只问一个问题）
│       ├── type          题型：choice(856条) / noul(888条) / score(932条)
│       ├── instructions  给模型看的规则说明书（政策原文，判断依据写在这）
│       └── criteria      每个选项 → 一段"选它的判定标准"文字
└── state[]               证据材料，逐条列出
    ├── speaker           这条证据的出处角色（如 "Account reviewer" 账户审核员）
    └── text              证据正文（一两句事实陈述）
```

- **三种题型在数据里的真实样子**：
  - `choice`（多选一）：criteria 的键是自定义选项名（如 `apply_credit_keep_annual` "补 24 美元并保持年费订阅"）；
  - `noul`（判断真假）：criteria 固定两个键 `"true"` / `"false"`，各配一段"什么证据下成立/不成立"；
  - `score`（打等级分）：criteria 的键是有序档位（1~5 档），每档写清评分标准。
- criteria 的**键的顺序**就是推理时分字母牌 A、B、C 的顺序（训练时会随机洗牌防止位置作弊）。
- `state` 故意设计成**必须两条证据联合才能推出答案**（质检项 `evidence_is_two_factual_sentences` 保证），单看任何一条都推不出——逼模型学"证据组合"而不是"关键词命中"。

## 三、`reference` —— 标准答案

| 字段 | 含义 |
|------|------|
| `target` | 正确选项（就是 criteria 里的某个键名 / true / false / 档位数字）。训练时它被换算成对应字母的 token，作为"该顶高分的那格" |
| `source` | 答案怎么来的：`audited_rule_over_verified_facts` = "经过审核的规则套经过验证的事实"，纯程序推导，不是人拍的，也不是大模型生成的 |
| `human_reviewed` | 是否人工复核过。全库 false——作者也明说了"标签全是合成的，模型自检可能犯同样的错" |

## 四、`evidence_certificate` —— 质检档案（数据最肥的部分）

造数流水线的每一步检查都留了痕，这是这套数据"敢自证质量"的核心。子块如下：

### 4.1 `spec` —— 出题蓝图（先设计后写文）
| 字段 | 含义 |
|------|------|
| `atoms[]` | 把政策拆成的**原子事实**清单：每条有 `id`（如 `offer_recorded` "有一笔有效的挽留优惠被记录在账户上"）和 `statement` 完整陈述。原子 = 一句话只说一件事 |
| `rules[]` | 判定规则：`when`（哪几个原子处于什么状态）→ `target`（该选哪个答案）。答案就是这套规则机械推出来的 |
| `base_states[]` / `counter_states[]` | 原版 / 改版本里每个原子的事实状态（supported 成立 / refuted 不成立 / unknown 不可知） |
| `focus_atom` | **本题的"命门"原子**——孪生对里唯一被改动的那个事实（如 `request_within_seven_days` "申请在七天之内"） |
| `focus_evidence[]` | 承载命门事实的那句证据（含它嵌在 state 里的路径 `path`） |
| `policy_evidence[]` | 承载政策规则的那句证据 |
| `base_state_json` | 原版证据全文快照（字符串形式，供重放） |

### 4.2 `verified_pair` —— 孪生证据对成品
| 字段 | 含义 |
|------|------|
| `left` / `right` | 组成答案必需的两句证据（左=政策句，右=事实句），如 right = "审计记录 AR-7319 的字段 E 值为 **6**" |
| `negative_left` / `negative_right` | 改一个词的孪生句："字段 E 值为 **9**"（6→9 跨过七天线，答案翻转） |

### 4.3 `fact_states` / `full_context_fact_states` —— 逐原子验真记录
- `fact_states`：本条数据里每个原子"应当"是什么状态（出题蓝图兑现表）。
- `full_context_fact_states`：**让另一个模型只读成文后的材料**，重新判断每个原子成立与否：
  - `base` / `counterfactual`：两个版本全文各自重判一遍——必须和蓝图一致，否则文字写砸了，弃用；
  - `remove_left` / `remove_right`：**抽掉左句/右句再重判**——命门原子必须变成 `unknown`。这是最狠的"防漏答案"检查：证明少了任何一句证据都推不出答案，题目没有捷径。

### 4.4 `pair_fact_states` —— 正/负样本对的交叉状态
`positive_pair`（原对：命门 supported）、`negative_pair`（改后对：命门 refuted）、`left`/`right`/`negative_*`（各拆半状态）——把 4.3 的结论按"对"的视角再存一份，供审计脚本直接比对。

### 4.5 `rule_audit` —— 规则本身的审查
| 字段 | 含义 |
|------|------|
| `atoms_are_atomic` | 每个原子确实只说一件事（没有一句话藏两个条件） |
| `assignments_are_realizable` | 规则里用到的每种事实组合都真实可构造（没有永远触发不到的死规则） |
| `policy_complete` | 政策覆盖了所有可能情形，不留"规则真空" |
| `focus_is_factual` | 命门改动是客观事实变更（改数字/人名），不是主观措辞 |
| `rule_checks[]` | 逐条规则的审查记录：`rule_index` + `sound`（推理是否可靠）+ `reason`（审查模型给的理由） |

### 4.6 `context_audit` —— 成文后的整体审查
| 字段 | 含义 |
|------|------|
| `counterfactual_is_coherent` | 改完数字后的版本读起来仍然自洽（不出现"6 天变 9 天但别处还写着 6 天"的矛盾） |
| `evidence_is_two_factual_sentences` | 答案确实需要两句事实证据 |
| `no_answer_leakage` | 文本里没有偷漏答案（没出现答案代号、结论句、"因此应选X"这类暗示） |
| `policy_preserved` | 孪生两版政策原文一字未动 |
| `question_bindings_preserved` | 两版的问题、账户、时间等绑定关系未动 |
| `explanation` | 审查模型写的整段说明文（人可读的质检报告） |

### 4.7 其余杂项
| 字段 | 含义 |
|------|------|
| `necessity_checks_passed` | 4.3 的"抽句即不可知"检验总开关：true 才入库 |
| `pipeline_version` | 造数流水线版本戳（`evidence-curation-v3`），方便复现 |
| `verifier_independent_model` | 审查用的模型和出题模型**是不是**两个独立模型（false = 同一个模型自查——作者承认的已知弱点：自己查不出自己的错） |

## 五、训练/推理各用到哪些字段

| 阶段 | 用到的字段 |
|------|-----------|
| 训练 | `input`（拼成 prompt）+ `reference.target`（换算成正确字母的 token，只对这个位置算损失）。**质检档案整个不参与训练**，纯留给审计 |
| 复现造数 | `evidence_certificate.spec` + `provenance`（可用同一套规则离线重放整个生成过程） |
| 洗牌防作弊 | 训练时按 `id` 做种子随机打乱 criteria 选项顺序（见 `schema_data.py`），所以"正确答案是第几格"每次训练不一样 |

## 六、统计速查

- 题型：score 932 / noul 888 / choice 856
- 孪生：base 1,338 + counterfactual 1,338（严格配对）
- 行业 10 类；标签 100% 合成、0% 人工复核
- 每条数据平均占用约 11KB，大头全在质检档案（题干本身只占几百字节）
