---
name: {{skill-name}}
description: {{One-line description of what this skill enforces}}
priority: medium
---
<!--
  HARNESS ENGINEERING · Skills
  
  Skill 目录结构（必须严格遵守）：
  
  docs/skills/
  └── {{skill-name}}/       ← 子目录，名字即为 skill ID
      └── SKILL.md          ← 本文件（必须命名为 SKILL.md）
  
  frontmatter 说明：
  - name:        skill 的唯一标识名（推荐与目录名一致）
  - description: 一行描述，显示在 zcode 加载时的提示
  - priority:    high / medium（默认） / low
  
  Body 写法：使用祈使句给 AI agent 下指令
  "Always...", "Never...", "Prefer..."
-->

## Rules

- [ ] **Rule 1:** {{Specific, verifiable rule}}
- [ ] **Rule 2:** {{Specific, verifiable rule}}
- [ ] **Rule 3:** {{Specific, verifiable rule}}

## Examples

### ✅ Correct

```{{language}}
{{correct example code}}
```

### ❌ Incorrect

```{{language}}
{{incorrect example code}}
```

## Rationale

{{Why this skill exists and what problems it prevents.}}
