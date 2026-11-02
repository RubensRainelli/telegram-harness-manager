/*
 * Personal and Educational Free License (PEF License / PEFL)
 * Version 1.0 - 2026-05-26
 * Copyright (c) 2026 Rubens Rainelli (rubens@rainelli.it)
 *
 * Personal and educational free use permitted.
 * Corporate, business or enterprise use prohibited without written authorization and fee.
 */

import { t } from '../i18n/index.js';

/**
 * Local Bridge HTTPS Endpoint (`tgcloud/endpoints/bridge.js`)
 *
 * SECURITY ARCHITECTURE:
 * End-user machines NEVER receive or know the Telegram Bot Token.
 * Instead, each paired machine authenticates with its personal `device_token`
 * (`Authorization: Bearer <device_token>`). This endpoint validates `device_token_hash`
 * against the SQLite `devices` table and scopes all queries strictly to `device.owner_user_id`.
 */
export default async function handleBridgeRequest(req, ctx) {
  const db = ctx.db;
  const body = typeof req.body === 'string' ? JSON.parse(req.body || '{}') : (req.body || {});
  const action = body.action || req.query?.action || '';
  const now = Math.floor(Date.now() / 1000);

  // 1. Unauthenticated action: Register a one-time Pairing Code from `telegram-harness-manager pair`
  if (action === 'register_pair') {
    const { code, deviceId, deviceName, deviceTokenHash, osName, harnesses } = body;
    if (!code || !deviceId || !deviceTokenHash) {
      return { status: 400, body: { ok: false, error: 'Missing pairing parameters' } };
    }
    await db.run(
      `INSERT INTO pairing_codes (code, device_id, device_name, device_token_hash, os_name, harnesses_json, expires_at, created_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
      [
        code,
        deviceId,
        deviceName || 'Developer PC',
        deviceTokenHash,
        osName || '',
        JSON.stringify(harnesses || ['antigravity', 'claude-code', 'opencode', 'deepseek']),
        now + 900,
        now
      ]
    );
    return { status: 200, body: { ok: true, expiresIn: 900 } };
  }

  // 2. All other actions require valid `Authorization: Bearer <device_token_hash>`
  const authHeader = req.headers?.authorization || req.headers?.Authorization || '';
  const tokenHash = authHeader.replace(/^Bearer\s+/i, '').trim() || body.deviceTokenHash || '';
  if (!tokenHash) {
    return { status: 401, body: { ok: false, error: 'Missing device token' } };
  }

  const device = await db.get(
    `SELECT * FROM devices WHERE device_token_hash = ? LIMIT 1`,
    [tokenHash]
  );
  if (!device) {
    return { status: 403, body: { ok: false, error: 'Device not paired or token revoked' } };
  }

  await db.run(`UPDATE devices SET last_seen_at = ? WHERE device_id = ?`, [now, device.device_id]);

  // 3. `poll`: Fetch approved queued prompts & resolved interactive questions for this Owner's device ONLY
  if (action === 'poll') {
    // Sync projects reported by the local daemon
    if (Array.isArray(body.projects)) {
      for (const p of body.projects) {
        await db.run(
          `INSERT INTO projects (project_id, owner_user_id, device_id, name, workspace_path, default_harness, default_model, default_thinking, default_permission_mode, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(project_id) DO UPDATE SET
             name = excluded.name,
             workspace_path = excluded.workspace_path,
             updated_at = excluded.updated_at`,
          [
            p.id,
            device.owner_user_id,
            device.device_id,
            p.name,
            p.workspace || '',
            p.harness || 'antigravity',
            p.model || 'gemini-4-argon',
            p.thinking || 'medium',
            p.permissions || 'ask',
            now
          ]
        );
      }
    }

    const prompts = await db.all(
      `SELECT * FROM prompt_queue
       WHERE device_id = ? AND owner_user_id = ? AND approval_status = 'approved' AND execution_status = 'queued'
       ORDER BY created_at ASC LIMIT 10`,
      [device.device_id, device.owner_user_id]
    );

    for (const prm of prompts || []) {
      await db.run(`UPDATE prompt_queue SET execution_status = 'running' WHERE id = ?`, [prm.id]);
    }

    const resolvedQuestions = await db.all(
      `SELECT id, conversation_id, decision_json FROM interactive_questions
       WHERE device_id = ? AND owner_user_id = ? AND resolved = 1`,
      [device.device_id, device.owner_user_id]
    );

    return {
      status: 200,
      body: {
        ok: true,
        ownerUserId: device.owner_user_id,
        prompts: prompts || [],
        resolvedQuestions: resolvedQuestions || []
      }
    };
  }

  // 4. `output`: Deliver Final Output when the Harness finishes a task (with Expandable Thinking & Steps)
  if (action === 'output') {
    const {
      promptId,
      conversationId,
      projectId,
      projectName,
      harness,
      outputText,
      thinkingText,
      stepsSummary,
      filesTouchedCount,
      gitDiff
    } = body;

    if (promptId) {
      await db.run(`UPDATE prompt_queue SET execution_status = 'completed' WHERE id = ? AND owner_user_id = ?`, [
        promptId,
        device.owner_user_id
      ]);
    }

    const conv = await db.get(
      `SELECT * FROM conversations WHERE conversation_id = ? AND owner_user_id = ?`,
      [conversationId, device.owner_user_id]
    );

    if (conv) {
      await db.run(
        `UPDATE conversations SET status = 'IDLE', last_output = ?, last_thinking = ?, last_steps_json = ?, last_diff = ?, updated_at = ?
         WHERE conversation_id = ?`,
        [
          outputText || '',
          thinkingText || '',
          JSON.stringify(stepsSummary || []),
          gitDiff || '',
          now,
          conversationId
        ]
      );
    }

    const ownerUser = await db.get(`SELECT language_code FROM users WHERE telegram_user_id = ?`, [device.owner_user_id]);
    const lang = ownerUser?.language_code || 'en';

    const header = t(lang, 'bot.task_completed_header', {
      project: escapeHtml(projectName || projectId || 'Project'),
      harness: escapeHtml(harness || conv?.harness || 'antigravity')
    });

    const stepsArr = Array.isArray(stepsSummary) ? stepsSummary : [];
    let expandableBlock = '';
    if (thinkingText || stepsArr.length > 0) {
      const expandTitle = t(lang, 'bot.steps_expandable_header', {
        steps_count: stepsArr.length || 1,
        files_count: filesTouchedCount || 0
      });
      const stepLines = stepsArr.slice(0, 12).map((s, i) => `${i + 1}. ${escapeHtml(s)}`).join('\n');
      const shortThinking = thinkingText ? `\n\n💭 ${escapeHtml(thinkingText.slice(0, 700))}` : '';
      expandableBlock = `\n\n<blockquote expandable><b>${expandTitle}</b>\n${stepLines}${shortThinking}</blockquote>`;
    }

    const fullHtml = `${header}\n\n${escapeHtml(outputText || 'Task completed.')}${expandableBlock}`;

    const targetChatId = conv?.group_chat_id || conv?.private_chat_id || device.owner_user_id;
    const targetThreadId = conv?.group_topic_id || conv?.private_topic_id || undefined;

    await ctx.bot.call('sendMessage', {
      chat_id: targetChatId,
      message_thread_id: targetThreadId,
      text: fullHtml.slice(0, 4050),
      parse_mode: 'HTML'
    });

    // Also broadcast to invited private collaborators (`viewer`, `proposer`, `executor`)
    if (projectId) {
      const guests = await db.all(`SELECT user_id FROM project_members WHERE project_id = ?`, [projectId]);
      for (const g of guests || []) {
        if (g.user_id !== device.owner_user_id && g.user_id !== targetChatId) {
          try {
            await ctx.bot.call('sendMessage', {
              chat_id: g.user_id,
              text: fullHtml.slice(0, 4050),
              parse_mode: 'HTML'
            });
          } catch (_) {}
        }
      }
    }

    return { status: 200, body: { ok: true } };
  }

  // 5. `ask`: Ask an Interactive Question (Single-Select, Multi-Select, or Tool Approval)
  if (action === 'ask') {
    const {
      questionId,
      conversationId,
      projectId,
      projectName,
      harness,
      kind, // 'single_select' | 'multi_select' | 'tool_approval'
      toolName,
      questionText,
      options
    } = body;

    const qId = questionId || `q_${Date.now()}`;
    const opts = Array.isArray(options) ? options : [];

    await db.run(
      `INSERT INTO interactive_questions (id, conversation_id, project_id, device_id, owner_user_id, kind, tool_name, question_text, options_json, selected_indices_json, resolved, created_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, '[]', 0, ?)`,
      [
        qId,
        conversationId || 'default',
        projectId || 'default',
        device.device_id,
        device.owner_user_id,
        kind || 'single_select',
        toolName || '',
        questionText || '',
        JSON.stringify(opts),
        now
      ]
    );

    const conv = await db.get(`SELECT * FROM conversations WHERE conversation_id = ?`, [conversationId]);
    const ownerUser = await db.get(`SELECT language_code FROM users WHERE telegram_user_id = ?`, [device.owner_user_id]);
    const lang = ownerUser?.language_code || 'en';

    let keyboard = [];
    let header = '';
    if (kind === 'tool_approval') {
      header = t(lang, 'bot.permission_request_header', {
        tool: escapeHtml(toolName || 'tool'),
        project: escapeHtml(projectName || projectId || 'Project')
      });
      keyboard = [
        [
          { text: t(lang, 'buttons.approve_once'), callback_data: `q_sel:${qId}:allow` },
          { text: t(lang, 'buttons.approve_session'), callback_data: `q_sel:${qId}:allow_session` }
        ],
        [
          { text: t(lang, 'buttons.reject'), callback_data: `q_sel:${qId}:deny` }
        ]
      ];
    } else if (kind === 'multi_select') {
      header = `${t(lang, 'bot.interactive_question_header', {
        harness: escapeHtml(harness || 'Harness'),
        project: escapeHtml(projectName || 'Project'),
        topic: escapeHtml(conv?.title || 'Session')
      })}\n${t(lang, 'bot.interactive_multi_hint')}`;
      keyboard = opts.map((opt, idx) => [
        { text: `⬜ ${opt}`, callback_data: `q_tog:${qId}:${idx}` }
      ]);
      keyboard.push([
        { text: t(lang, 'buttons.confirm_multi', { count: 0 }), callback_data: `q_sub:${qId}:confirm` }
      ]);
    } else {
      header = `${t(lang, 'bot.interactive_question_header', {
        harness: escapeHtml(harness || 'Harness'),
        project: escapeHtml(projectName || 'Project'),
        topic: escapeHtml(conv?.title || 'Session')
      })}\n${t(lang, 'bot.interactive_single_hint')}`;
      keyboard = opts.map((opt, idx) => [
        { text: `${idx === 0 && opt.toLowerCase().includes('recommend') ? '⭐ ' : ''}${opt}`, callback_data: `q_sel:${qId}:${idx}` }
      ]);
    }

    const targetChatId = conv?.group_chat_id || conv?.private_chat_id || device.owner_user_id;
    const targetThreadId = conv?.group_topic_id || conv?.private_topic_id || undefined;

    await ctx.bot.call('sendMessage', {
      chat_id: targetChatId,
      message_thread_id: targetThreadId,
      text: `${header}\n\n<blockquote>${escapeHtml(questionText)}</blockquote>`,
      parse_mode: 'HTML',
      reply_markup: { inline_keyboard: keyboard }
    });

    return { status: 200, body: { ok: true, questionId: qId } };
  }

  return { status: 400, body: { ok: false, error: `Unknown bridge action '${action}'` } };
}

function escapeHtml(str) {
  return String(str || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}
