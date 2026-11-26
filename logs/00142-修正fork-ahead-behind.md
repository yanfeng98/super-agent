# 00142 — 修正 GitHub fork ahead/behind 显示

- **日期**：2026-11-26
- **背景**：GitHub 上 `yanfeng98/super-agent:main` 相对 `deepseek-ai/deepseek-harness:master` 显示 9290 ahead / 8923 behind。fork 自建立以来历次「同步上游」都是把上游快照以新 SHA 重新提交（`chore(sync)` 系列），导致两边虽有 20049 个共同祖先、树内容逐一相同，但提交对象不同，GitHub 判定为大幅分叉。
- **判断依据**：统计 `main --not upstream/master` 得到 9290 个提交，其中 28972 个非 yanfeng98 提交的 tree 与 upstream/master 提交逐一相同，367 个由 `yanfeng98 <3555028709@qq.com>` 创作（另有 3 个 GitHub 网页编辑用 `60573679+yanfeng98@users.noreply.github.com`，同属本地改动）。因此可把 fork 的非本地提交整体替换为真实上游提交，只重建本地提交。
- **做法**：按 (tree, author, email, date, subject) 建立 fork 非本地提交 → 上游提交的映射（28969 个精确匹配，6 个歧义用同 SHA 优先消解，3 个网页编辑归入本地集合）。用 `git commit-tree` 按拓扑序重建 370 个本地提交：保留原始 tree、作者、提交者、时间与完整 message，仅把指向 fork 副本的父提交替换为映射后的真实上游提交。新 tip `1bc2afb259`，树与原 main 完全一致（`885f17edd66`）。用 `--force-with-lease=refs/heads/main:0ce0aa0eae` 推送到 origin/main，旧 tip 保留为 `backup/fork-compare-original` 标签。
- **结果**：`origin/main...upstream/master` = 367 ahead / 0 behind；上游提交保持原作者与原 SHA，367 个 ahead 提交作者均为 yanfeng98；工作树内容零变化。
- **验证**：`git fsck --connectivity-only` 无损坏；`git diff main <new-tip>` 为空；首个重建提交的 tree/作者/日期/message 与原提交逐一相同；远端 `git ls-remote` 确认 main 已更新。
