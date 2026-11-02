/*
 * Personal and Educational Free License (PEF License / PEFL)
 * Version 1.0 - 2026-05-26
 * Copyright (c) 2026 Rubens Rainelli (rubens@rainelli.it)
 *
 * Personal and educational free use permitted.
 * Corporate, business or enterprise use prohibited without written authorization and fee.
 */

import { t, resolveLocale } from '../i18n/index.js';

export const HARNESS_MODELS = {
  'antigravity': ['gemini-4-argon', 'claude-sonnet-4', 'claude-opus-4', 'gpt-5'],
  'claude-code': ['claude-sonnet-4', 'claude-opus-4', 'claude-haiku-4'],
  'opencode': ['anthropic/claude-sonnet-4', 'google/gemini-2.5-pro', 'openai/o3', 'deepseek/deepseek-reasoner'],
  'deepseek': ['deepseek-reasoner', 'deepseek-chat'],
  'aider': ['sonnet', 'deepseek', 'o3-mini']
};

export const THINKING_LEVELS = ['none', 'low', 'medium', 'high', 'max'];
export const PERMISSION_MODES = ['plan', 'ask', 'auto_edit', 'auto'];

/**
 * Telegram Serverless Webhook Handler (`tgcloud/bot/webhook.js`)
 * Notice: The Bot Token stays strictly inside Telegram Serverless (`ctx.bot`)
 * and is NEVER exposed to end-user machines.
 */
export default async function handleWebhook(update, ctx) {
  const db = ctx.db;

  // 1. Handle Bot Added / Updated in a Group (`my_chat_member`)
  if (update.my_chat_member) {
    return await handleGroupBotMembership(update.my_chat_member, ctx, db);
  }

  // 2. Handle Inline Keyboard Callback Queries
  if (update.callback_query) {
    return await handleCallbackQuery(update.callback_query, ctx, db);
  }

  // 3. Handle Incoming Messages (Commands, Prompts in Conversation Topics, Custom Replies)
  if (update.message) {
    return await handleMessage(update.message, ctx, db);
  }
}

async function upsertUser(from, db) {
  if (!from) return 'en';
  const lang = resolveLocale(from.language_code);
  const now = Math.floor(Date.now() / 1000);
  await db.run(
    `INSERT INTO users (telegram_user_id, username, first_name, language_code, created_at)
     VALUES (?, ?, ?, ?, ?)
     ON CONFLICT(telegram_user_id) DO UPDATE SET
       username = excluded.username,
       first_name = excluded.first_name,
       language_code = excluded.language_code`,
    [from.id, from.username || '', from.first_name || '', lang, now]
  );
  return lang;
}

/**
 * Group Pre-Flight Security Check:
 * 1. Only a verified Harness Owner (with at least 1 paired device) can add/link the bot to a group.
 * 2. Group MUST have Forum Topics enabled (`chat.is_forum === true`).
 * 3. Bot MUST be an Administrator with `can_manage_topics === true`.
 * 4. Default group member role is strictly `viewer` (Read-Only) to prevent unauthorized prompt execution.
 */
async function handleGroupBotMembership(memberUpdate, ctx, db) {
  const chat = memberUpdate.chat;
  const from = memberUpdate.from;
  if (!chat || (chat.type !== 'supergroup' && chat.type !== 'group')) return;

  const newStatus = memberUpdate.new_chat_member?.status;
  if (newStatus === 'left' || newStatus === 'kicked') return;

  const lang = await upsertUser(from, db);

  // Verify that `from.id` is an Owner of at least one paired Harness device
  const ownedDevice = await db.get(
    `SELECT device_id FROM devices WHERE owner_user_id = ? LIMIT 1`,
    [from.id]
  );

  if (!ownedDevice) {
    await ctx.bot.call('sendMessage', {
      chat_id: chat.id,
      text: t(lang, 'bot.group_not_owner'),
      parse_mode: 'HTML'
    });
    // Leave group to protect security
    await ctx.bot.call('leaveChat', { chat_id: chat.id });
    return;
  }

  await verifyAndConfigureGroup(chat.id, from.id, lang, ctx, db);
}

async function verifyAndConfigureGroup(chatId, ownerUserId, lang, ctx, db) {
  const chatInfo = await ctx.bot.call('getChat', { chat_id: chatId });
  const me = await ctx.bot.call('getMe', {});
  const botMember = await ctx.bot.call('getChatMember', {
    chat_id: chatId,
    user_id: me.id
  });

  const isForum = Boolean(chatInfo?.is_forum);
  const isAdmin = botMember?.status === 'administrator';
  const canManageTopics = Boolean(botMember?.can_manage_topics);

  const now = Math.floor(Date.now() / 1000);
  const defaultProject = await db.get(
    `SELECT project_id FROM projects WHERE owner_user_id = ? ORDER BY updated_at DESC LIMIT 1`,
    [ownerUserId]
  );

  await db.run(
    `INSERT INTO group_bindings (group_chat_id, project_id, owner_user_id, default_member_role, topics_enabled, bot_can_manage_topics, updated_at)
     VALUES (?, ?, ?, 'viewer', ?, ?, ?)
     ON CONFLICT(group_chat_id) DO UPDATE SET
       topics_enabled = excluded.topics_enabled,
       bot_can_manage_topics = excluded.bot_can_manage_topics,
       updated_at = excluded.updated_at`,
    [chatId, defaultProject?.project_id || '', ownerUserId, isForum ? 1 : 0, (isAdmin && canManageTopics) ? 1 : 0, now]
  );

  if (!isForum || !isAdmin || !canManageTopics) {
    await ctx.bot.call('sendMessage', {
      chat_id: chatId,
      text: t(lang, 'bot.group_check_failed', {
        topics_status: isForum ? '✅ OK' : '❌ Disabled (Enable Topics in Group Settings)',
        admin_status: isAdmin ? '✅ OK' : '❌ Bot is not Admin',
        manage_topics_status: canManageTopics ? '✅ OK' : '❌ Missing "Manage Topics" permission'
      }),
      parse_mode: 'HTML',
      reply_markup: {
        inline_keyboard: [
          [{ text: t(lang, 'buttons.recheck_group'), callback_data: `grp_recheck:${chatId}` }]
        ]
      }
    });
    return false;
  }

  await ctx.bot.call('sendMessage', {
    chat_id: chatId,
    text: t(lang, 'bot.group_setup_ok'),
    parse_mode: 'HTML',
    reply_markup: {
      inline_keyboard: [
        [
          { text: '👁️ Default: Viewer (Selected)', callback_data: `grp_defrole:${chatId}:viewer` },
          { text: '✋ Default: Proposer', callback_data: `grp_defrole:${chatId}:proposer` }
        ],
        [
          { text: t(lang, 'buttons.new_conversation'), callback_data: `new_conv_menu` }
        ]
      ]
    }
  });
  return true;
}

async function handleMessage(msg, ctx, db) {
  const from = msg.from;
  if (!from || from.is_bot) return;

  const lang = await upsertUser(from, db);
  const text = (msg.text || msg.caption || '').trim();
  const chatId = msg.chat.id;
  const threadId = msg.message_thread_id || null;

  // 1. Check `/start` (Pairing & Collaborator Invites)
  if (text.startsWith('/start')) {
    const parts = text.split(/\s+/);
    const payload = parts[1] || '';

    if (payload.startsWith('pair_')) {
      const code = payload.slice(5);
      return await handleDevicePairing(chatId, from, code, lang, ctx, db);
    }

    if (payload.startsWith('invite_')) {
      const token = payload.slice(7);
      return await handleAcceptInvite(chatId, from, token, lang, ctx, db);
    }

    return await sendControlCenter(chatId, threadId, from, lang, ctx, db);
  }

  // 2. Check `/control` or `/projects` or `/setup`
  if (text === '/control' || text === '/projects' || text === '/settings') {
    return await sendControlCenter(chatId, threadId, from, lang, ctx, db);
  }

  if (text === '/setup_group' && (msg.chat.type === 'supergroup' || msg.chat.type === 'group')) {
    const ownedDevice = await db.get(`SELECT device_id FROM devices WHERE owner_user_id = ? LIMIT 1`, [from.id]);
    if (!ownedDevice) {
      return await ctx.bot.call('sendMessage', {
        chat_id: chatId,
        message_thread_id: threadId,
        text: t(lang, 'bot.group_not_owner'),
        parse_mode: 'HTML'
      });
    }
    return await verifyAndConfigureGroup(chatId, from.id, lang, ctx, db);
  }

  // 3. Check if this message is inside a Conversation Topic (Private Threaded Mode or Group Forum Topic)
  let conv = null;
  if (threadId) {
    conv = await db.get(
      `SELECT * FROM conversations WHERE (private_chat_id = ? AND private_topic_id = ?) OR (group_chat_id = ? AND group_topic_id = ?) LIMIT 1`,
      [chatId, threadId, chatId, threadId]
    );
  }
  if (!conv && msg.chat.type === 'private') {
    // Fallback to most recently active conversation of the user if private topics are not enabled yet
    conv = await db.get(
      `SELECT c.* FROM conversations c
       LEFT JOIN project_members pm ON pm.project_id = c.project_id AND pm.user_id = ?
       WHERE c.owner_user_id = ? OR pm.user_id = ?
       ORDER BY c.updated_at DESC LIMIT 1`,
      [from.id, from.id, from.id]
    );
  }

  if (!conv) {
    return await sendControlCenter(chatId, threadId, from, lang, ctx, db);
  }

  // 4. Check User's Collaborator Role on this Project (`owner` | `executor` | `proposer` | `viewer`)
  const roleInfo = await resolveUserProjectRole(from.id, conv.project_id, msg.chat, db);
  if (roleInfo.role === 'viewer') {
    // In group chats, allow normal conversation without error spam unless they explicitly tag the bot
    if (msg.chat.type === 'private' || text.includes('@')) {
      await ctx.bot.call('sendMessage', {
        chat_id: chatId,
        message_thread_id: threadId,
        text: t(lang, 'bot.viewer_denied'),
        parse_mode: 'HTML'
      });
    }
    return;
  }

  // 5. Parse optional inline prompt flags (e.g., `!high !plan Refactor auth`)
  const parsedPrompt = parsePromptOverrides(text, conv, roleInfo);
  const project = await db.get(`SELECT * FROM projects WHERE project_id = ?`, [conv.project_id]);
  const projectName = project?.name || conv.project_id;
  const now = Math.floor(Date.now() / 1000);
  const promptId = `prm_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;

  // 6. If user is a `proposer`, hold prompt and ask Project Owner for manual approval!
  if (roleInfo.role === 'proposer') {
    await db.run(
      `INSERT INTO prompt_queue (id, conversation_id, project_id, device_id, owner_user_id, author_user_id, author_name, prompt_text, harness, model, thinking_level, permission_mode, approval_status, execution_status, created_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending_owner', 'queued', ?)`,
      [
        promptId,
        conv.conversation_id,
        conv.project_id,
        conv.device_id,
        conv.owner_user_id,
        from.id,
        from.username ? `@${from.username}` : from.first_name,
        parsedPrompt.cleanText,
        parsedPrompt.harness,
        parsedPrompt.model,
        parsedPrompt.thinking,
        parsedPrompt.permissions,
        now
      ]
    );

    await ctx.bot.call('sendMessage', {
      chat_id: chatId,
      message_thread_id: threadId,
      text: t(lang, 'bot.prompt_held_for_approval', { project: projectName }),
      parse_mode: 'HTML'
    });

    // Notify Project Owner with interactive Approval Card
    const ownerUser = await db.get(`SELECT language_code FROM users WHERE telegram_user_id = ?`, [conv.owner_user_id]);
    const ownerLang = ownerUser?.language_code || 'en';
    await ctx.bot.call('sendMessage', {
      chat_id: conv.owner_user_id,
      text: t(ownerLang, 'bot.owner_approval_card', {
        author: from.username ? `@${from.username}` : from.first_name,
        project: projectName,
        topic: conv.title,
        model: parsedPrompt.model,
        thinking: parsedPrompt.thinking,
        permissions: parsedPrompt.permissions,
        prompt: escapeHtml(parsedPrompt.cleanText)
      }),
      parse_mode: 'HTML',
      reply_markup: {
        inline_keyboard: [
          [
            { text: t(ownerLang, 'buttons.owner_approve_run'), callback_data: `own_apr:${promptId}:approve` },
            { text: '📋 Plan-Only', callback_data: `own_apr:${promptId}:plan` }
          ],
          [
            { text: t(ownerLang, 'buttons.owner_reject'), callback_data: `own_apr:${promptId}:reject` }
          ]
        ]
      }
    });
    return;
  }

  // 7. User is `owner` or `executor`: Queue prompt immediately for the Local Harness Daemon
  await db.run(
    `INSERT INTO prompt_queue (id, conversation_id, project_id, device_id, owner_user_id, author_user_id, author_name, prompt_text, harness, model, thinking_level, permission_mode, approval_status, execution_status, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'approved', 'queued', ?)`,
    [
      promptId,
      conv.conversation_id,
      conv.project_id,
      conv.device_id,
      conv.owner_user_id,
      from.id,
      from.username ? `@${from.username}` : from.first_name,
      parsedPrompt.cleanText,
      parsedPrompt.harness,
      parsedPrompt.model,
      parsedPrompt.thinking,
      parsedPrompt.permissions,
      now
    ]
  );

  // Acknowledge quietly without spamming live thoughts
  await ctx.bot.call('sendMessage', {
    chat_id: chatId,
    message_thread_id: threadId,
    text: t(lang, 'bot.prompt_queued_owner', {
      harness: parsedPrompt.harness,
      model: parsedPrompt.model,
      thinking: parsedPrompt.thinking,
      permissions: parsedPrompt.permissions
    }),
    parse_mode: 'HTML'
  });
}

async function handleDevicePairing(chatId, from, code, lang, ctx, db) {
  const now = Math.floor(Date.now() / 1000);
  const pairing = await db.get(
    `SELECT * FROM pairing_codes WHERE code = ? AND expires_at > ? AND claimed_by_user_id IS NULL`,
    [code, now]
  );

  if (!pairing) {
    return await ctx.bot.call('sendMessage', {
      chat_id: chatId,
      text: t(lang, 'bot.pair_invalid'),
      parse_mode: 'HTML'
    });
  }

  await db.run(`UPDATE pairing_codes SET claimed_by_user_id = ? WHERE code = ?`, [from.id, code]);
  await db.run(
    `INSERT INTO devices (device_id, owner_user_id, device_name, device_token_hash, os_name, harnesses_json, last_seen_at, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)
     ON CONFLICT(device_id) DO UPDATE SET
       owner_user_id = excluded.owner_user_id,
       device_token_hash = excluded.device_token_hash,
       last_seen_at = excluded.last_seen_at`,
    [
      pairing.device_id,
      from.id,
      pairing.device_name,
      pairing.device_token_hash,
      pairing.os_name,
      pairing.harnesses_json,
      now,
      now
    ]
  );

  await ctx.bot.call('sendMessage', {
    chat_id: chatId,
    text: t(lang, 'bot.pair_success', {
      device_name: escapeHtml(pairing.device_name),
      user_id: from.id
    }),
    parse_mode: 'HTML'
  });

  return await sendControlCenter(chatId, null, from, lang, ctx, db);
}

async function handleAcceptInvite(chatId, from, token, lang, ctx, db) {
  const now = Math.floor(Date.now() / 1000);
  const invite = await db.get(
    `SELECT * FROM project_invites WHERE invite_token = ? AND expires_at > ?`,
    [token, now]
  );
  if (!invite) {
    return await ctx.bot.call('sendMessage', {
      chat_id: chatId,
      text: t(lang, 'bot.pair_invalid'),
      parse_mode: 'HTML'
    });
  }

  await db.run(
    `INSERT INTO project_members (project_id, user_id, username, role, max_permission_mode, invited_by, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?)
     ON CONFLICT(project_id, user_id) DO UPDATE SET
       role = excluded.role,
       max_permission_mode = excluded.max_permission_mode`,
    [
      invite.project_id,
      from.id,
      from.username || from.first_name || String(from.id),
      invite.role,
      invite.max_permission_mode,
      invite.owner_user_id,
      now
    ]
  );

  const project = await db.get(`SELECT name FROM projects WHERE project_id = ?`, [invite.project_id]);
  await ctx.bot.call('sendMessage', {
    chat_id: chatId,
    text: t(lang, 'bot.invite_accepted', {
      role: t(lang, `roles.${invite.role}`),
      project: project?.name || invite.project_id
    }),
    parse_mode: 'HTML'
  });
}

async function sendControlCenter(chatId, threadId, from, lang, ctx, db) {
  const projects = await db.all(
    `SELECT DISTINCT p.* FROM projects p
     LEFT JOIN project_members pm ON pm.project_id = p.project_id AND pm.user_id = ?
     WHERE p.owner_user_id = ? OR pm.user_id = ?
     ORDER BY p.updated_at DESC LIMIT 15`,
    [from.id, from.id, from.id]
  );

  const keyboard = [];
  for (const p of projects || []) {
    const isOwner = p.owner_user_id === from.id;
    keyboard.push([
      {
        text: `📁 ${p.name} (${p.default_harness})`,
        callback_data: `proj_open:${p.project_id}`
      },
      ...(isOwner
        ? [{ text: '👥 Guests', callback_data: `proj_guests:${p.project_id}` }]
        : [])
    ]);
  }

  keyboard.push([
    { text: t(lang, 'buttons.new_conversation'), callback_data: 'new_conv_menu' }
  ]);

  await ctx.bot.call('sendMessage', {
    chat_id: chatId,
    message_thread_id: threadId,
    text: `${t(lang, 'bot.welcome_title')}\n\n${t(lang, 'bot.welcome_body')}\n\n${t(lang, 'bot.control_center_body')}`,
    parse_mode: 'HTML',
    reply_markup: { inline_keyboard: keyboard }
  });
}

async function handleCallbackQuery(cb, ctx, db) {
  const from = cb.from;
  const lang = await upsertUser(from, db);
  const data = cb.data || '';
  const chatId = cb.message?.chat?.id;
  const messageId = cb.message?.message_id;
  const threadId = cb.message?.message_thread_id || null;

  // 1. Group permissions re-check
  if (data.startsWith('grp_recheck:')) {
    const grpId = Number(data.split(':')[1]);
    await verifyAndConfigureGroup(grpId, from.id, lang, ctx, db);
    await ctx.bot.call('answerCallbackQuery', { callback_query_id: cb.id });
    return;
  }

  // 2. Group Default Member Role update (Owner only)
  if (data.startsWith('grp_defrole:')) {
    const [, grpIdStr, newRole] = data.split(':');
    const binding = await db.get(`SELECT * FROM group_bindings WHERE group_chat_id = ?`, [Number(grpIdStr)]);
    if (binding && binding.owner_user_id === from.id) {
      await db.run(`UPDATE group_bindings SET default_member_role = ? WHERE group_chat_id = ?`, [newRole, Number(grpIdStr)]);
      await ctx.bot.call('answerCallbackQuery', {
        callback_query_id: cb.id,
        text: `Default role updated to: ${newRole}`
      });
    } else {
      await ctx.bot.call('answerCallbackQuery', {
        callback_query_id: cb.id,
        text: t(lang, 'bot.group_not_owner'),
        show_alert: true
      });
    }
    return;
  }

  // 3. Create a New Conversation Topic for a Project (1 Project -> N Conversation Topics)
  if (data.startsWith('proj_open:') || data.startsWith('new_conv:')) {
    const projectId = data.split(':')[1];
    const project = await db.get(`SELECT * FROM projects WHERE project_id = ?`, [projectId]);
    if (!project) return;

    const convId = `conv_${Date.now().toString(36)}`;
    const topicTitle = `📁 [${project.name}] 💬 Session #${convId.slice(-4)}`;
    let createdTopicId = threadId;

    try {
      const topicRes = await ctx.bot.call('createForumTopic', {
        chat_id: chatId,
        name: topicTitle
      });
      if (topicRes?.message_thread_id) {
        createdTopicId = topicRes.message_thread_id;
      }
    } catch (_) {
      // Fallback if private Threaded Mode is not active yet
    }

    const now = Math.floor(Date.now() / 1000);
    await db.run(
      `INSERT INTO conversations (conversation_id, project_id, owner_user_id, device_id, title, harness, model, thinking_level, permission_mode, status, private_chat_id, private_topic_id, updated_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'IDLE', ?, ?, ?)`,
      [
        convId,
        project.project_id,
        project.owner_user_id,
        project.device_id,
        topicTitle,
        project.default_harness,
        project.default_model,
        project.default_thinking,
        project.default_permission_mode,
        chatId,
        createdTopicId,
        now
      ]
    );

    const conv = await db.get(`SELECT * FROM conversations WHERE conversation_id = ?`, [convId]);
    await sendOrUpdateTopicConfigBar(chatId, createdTopicId, null, conv, project.name, lang, ctx);
    await ctx.bot.call('answerCallbackQuery', { callback_query_id: cb.id, text: topicTitle });
    return;
  }

  // 4. Cycle Harness / Model / Thinking Level / Permission Mode on a Conversation Topic
  if (data.startsWith('cfg_cycle:')) {
    const [, convId, field] = data.split(':');
    const conv = await db.get(`SELECT * FROM conversations WHERE conversation_id = ?`, [convId]);
    if (!conv) return;

    const roleInfo = await resolveUserProjectRole(from.id, conv.project_id, cb.message.chat, db);
    if (roleInfo.role === 'viewer') {
      return await ctx.bot.call('answerCallbackQuery', {
        callback_query_id: cb.id,
        text: t(lang, 'bot.viewer_denied'),
        show_alert: true
      });
    }

    let newHarness = conv.harness;
    let newModel = conv.model;
    let newThinking = conv.thinking_level;
    let newPerms = conv.permission_mode;

    if (field === 'harness') {
      const harnesses = Object.keys(HARNESS_MODELS);
      newHarness = harnesses[(harnesses.indexOf(conv.harness) + 1) % harnesses.length];
      newModel = HARNESS_MODELS[newHarness][0];
    } else if (field === 'model') {
      const models = HARNESS_MODELS[conv.harness] || ['default'];
      newModel = models[(models.indexOf(conv.model) + 1) % models.length];
    } else if (field === 'thinking') {
      newThinking = THINKING_LEVELS[(THINKING_LEVELS.indexOf(conv.thinking_level) + 1) % THINKING_LEVELS.length];
    } else if (field === 'permissions') {
      newPerms = PERMISSION_MODES[(PERMISSION_MODES.indexOf(conv.permission_mode) + 1) % PERMISSION_MODES.length];
    }

    await db.run(
      `UPDATE conversations SET harness = ?, model = ?, thinking_level = ?, permission_mode = ?, updated_at = ? WHERE conversation_id = ?`,
      [newHarness, newModel, newThinking, newPerms, Math.floor(Date.now() / 1000), convId]
    );

    const updatedConv = await db.get(`SELECT * FROM conversations WHERE conversation_id = ?`, [convId]);
    const project = await db.get(`SELECT name FROM projects WHERE project_id = ?`, [conv.project_id]);
    await sendOrUpdateTopicConfigBar(chatId, threadId, messageId, updatedConv, project?.name || conv.project_id, lang, ctx);
    await ctx.bot.call('answerCallbackQuery', { callback_query_id: cb.id });
    return;
  }

  // 5. Generate Collaborator Invite Links (`viewer` | `proposer` | `executor`)
  if (data.startsWith('proj_guests:')) {
    const projectId = data.split(':')[1];
    const project = await db.get(`SELECT * FROM projects WHERE project_id = ? AND owner_user_id = ?`, [projectId, from.id]);
    if (!project) return;

    await ctx.bot.call('sendMessage', {
      chat_id: chatId,
      message_thread_id: threadId,
      text: `👥 <b>${escapeHtml(project.name)} — Collaborator Roles</b>\nChoose the permission level to generate an invite link:`,
      parse_mode: 'HTML',
      reply_markup: {
        inline_keyboard: [
          [{ text: t(lang, 'roles.viewer'), callback_data: `inv_gen:${projectId}:viewer` }],
          [{ text: t(lang, 'roles.proposer'), callback_data: `inv_gen:${projectId}:proposer` }],
          [{ text: t(lang, 'roles.executor'), callback_data: `inv_gen:${projectId}:executor` }]
        ]
      }
    });
    await ctx.bot.call('answerCallbackQuery', { callback_query_id: cb.id });
    return;
  }

  if (data.startsWith('inv_gen:')) {
    const [, projectId, role] = data.split(':');
    const project = await db.get(`SELECT * FROM projects WHERE project_id = ? AND owner_user_id = ?`, [projectId, from.id]);
    if (!project) return;

    const token = `${projectId.slice(-4)}_${Math.random().toString(36).slice(2, 10)}`;
    const now = Math.floor(Date.now() / 1000);
    await db.run(
      `INSERT INTO project_invites (invite_token, project_id, owner_user_id, role, max_permission_mode, expires_at, created_at)
       VALUES (?, ?, ?, ?, ?, ?, ?)`,
      [token, projectId, from.id, role, role === 'executor' ? 'auto_edit' : 'ask', now + 86400 * 7, now]
    );

    const me = await ctx.bot.call('getMe', {});
    const link = `https://t.me/${me.username}?start=invite_${token}`;
    await ctx.bot.call('sendMessage', {
      chat_id: chatId,
      message_thread_id: threadId,
      text: t(lang, 'bot.invite_created', {
        project: escapeHtml(project.name),
        role: t(lang, `roles.${role}`),
        link
      }),
      parse_mode: 'HTML'
    });
    await ctx.bot.call('answerCallbackQuery', { callback_query_id: cb.id });
    return;
  }

  // 6. Owner Manual Approval for Proposer's Prompt (`own_apr:<promptId>:<action>`)
  if (data.startsWith('own_apr:')) {
    const [, promptId, action] = data.split(':');
    const item = await db.get(`SELECT * FROM prompt_queue WHERE id = ? AND owner_user_id = ?`, [promptId, from.id]);
    if (!item) return;

    if (action === 'reject') {
      await db.run(`UPDATE prompt_queue SET approval_status = 'rejected', execution_status = 'failed' WHERE id = ?`, [promptId]);
      await ctx.bot.call('editMessageText', {
        chat_id: chatId,
        message_id: messageId,
        text: `❌ Prompt from ${escapeHtml(item.author_name)} rejected.`
      });
    } else {
      const newPerm = action === 'plan' ? 'plan' : item.permission_mode;
      await db.run(
        `UPDATE prompt_queue SET approval_status = 'approved', permission_mode = ? WHERE id = ?`,
        [newPerm, promptId]
      );
      await ctx.bot.call('editMessageText', {
        chat_id: chatId,
        message_id: messageId,
        text: `✅ Approved & Dispatched prompt from ${escapeHtml(item.author_name)} (Mode: ${newPerm}).`
      });
    }
    await ctx.bot.call('answerCallbackQuery', { callback_query_id: cb.id });
    return;
  }

  // 7. Interactive Harness Questions (Single-Select, Multi-Select Toggle, Tool Approvals)
  if (data.startsWith('q_sel:') || data.startsWith('q_tog:') || data.startsWith('q_sub:')) {
    const [, qId, val] = data.split(':');
    const q = await db.get(`SELECT * FROM interactive_questions WHERE id = ?`, [qId]);
    if (!q || q.resolved) {
      return await ctx.bot.call('answerCallbackQuery', { callback_query_id: cb.id, text: 'Already resolved' });
    }

    const options = JSON.parse(q.options_json || '[]');
    let selected = JSON.parse(q.selected_indices_json || '[]');

    if (data.startsWith('q_tog:')) {
      const idx = Number(val);
      if (selected.includes(idx)) {
        selected = selected.filter((i) => i !== idx);
      } else {
        selected.push(idx);
      }
      await db.run(`UPDATE interactive_questions SET selected_indices_json = ? WHERE id = ?`, [JSON.stringify(selected), qId]);

      const kb = options.map((opt, idx) => [
        {
          text: `${selected.includes(idx) ? '☑️' : '⬜'} ${opt}`,
          callback_data: `q_tog:${qId}:${idx}`
        }
      ]);
      kb.push([
        {
          text: t(lang, 'buttons.confirm_multi', { count: selected.length }),
          callback_data: `q_sub:${qId}:confirm`
        }
      ]);

      await ctx.bot.call('editMessageReplyMarkup', {
        chat_id: chatId,
        message_id: messageId,
        reply_markup: { inline_keyboard: kb }
      });
      await ctx.bot.call('answerCallbackQuery', { callback_query_id: cb.id });
      return;
    }

    // Resolve Single-Select, Multi-Select Submit, or Tool Approval
    let decisionPayload = {};
    if (data.startsWith('q_sub:')) {
      decisionPayload = {
        selectedIndices: selected,
        selectedLabels: selected.map((i) => options[i]).filter(Boolean),
        allow: true
      };
    } else if (q.kind === 'tool_approval') {
      decisionPayload = {
        allow: val === 'allow' || val === 'allow_session',
        sessionAllow: val === 'allow_session',
        reason: val === 'deny' ? 'Rejected via Telegram' : 'Approved via Telegram'
      };
    } else {
      const idx = Number(val);
      decisionPayload = {
        selectedIndices: [idx],
        selectedLabels: [options[idx] || val],
        allow: true
      };
    }

    await db.run(
      `UPDATE interactive_questions SET resolved = 1, decision_json = ? WHERE id = ?`,
      [JSON.stringify(decisionPayload), qId]
    );

    await ctx.bot.call('answerCallbackQuery', { callback_query_id: cb.id, text: '✅ Sent to Harness' });
  }
}

async function sendOrUpdateTopicConfigBar(chatId, threadId, messageId, conv, projectName, lang, ctx) {
  const text = t(lang, 'bot.topic_config_bar', {
    project: escapeHtml(projectName),
    title: escapeHtml(conv.title),
    harness: conv.harness,
    model: conv.model,
    thinking: t(lang, `thinking_levels.${conv.thinking_level}`),
    permissions: t(lang, `permission_modes.${conv.permission_mode}`)
  });

  const reply_markup = {
    inline_keyboard: [
      [
        { text: t(lang, 'buttons.change_harness', { value: conv.harness }), callback_data: `cfg_cycle:${conv.conversation_id}:harness` },
        { text: t(lang, 'buttons.change_model', { value: conv.model }), callback_data: `cfg_cycle:${conv.conversation_id}:model` }
      ],
      [
        { text: t(lang, 'buttons.change_thinking', { value: conv.thinking_level }), callback_data: `cfg_cycle:${conv.conversation_id}:thinking` },
        { text: t(lang, 'buttons.change_permissions', { value: conv.permission_mode }), callback_data: `cfg_cycle:${conv.conversation_id}:permissions` }
      ]
    ]
  };

  if (messageId) {
    await ctx.bot.call('editMessageText', {
      chat_id: chatId,
      message_id: messageId,
      text,
      parse_mode: 'HTML',
      reply_markup
    });
  } else {
    const sent = await ctx.bot.call('sendMessage', {
      chat_id: chatId,
      message_thread_id: threadId,
      text,
      parse_mode: 'HTML',
      reply_markup
    });
    if (sent?.message_id) {
      try {
        await ctx.bot.call('pinChatMessage', {
          chat_id: chatId,
          message_id: sent.message_id,
          disable_notification: true
        });
      } catch (_) {}
    }
  }
}

async function resolveUserProjectRole(userId, projectId, chat, db) {
  const project = await db.get(`SELECT * FROM projects WHERE project_id = ?`, [projectId]);
  if (project && project.owner_user_id === userId) {
    return { role: 'owner', maxPermission: 'auto' };
  }
  const member = await db.get(
    `SELECT * FROM project_members WHERE project_id = ? AND user_id = ?`,
    [projectId, userId]
  );
  if (member) {
    return { role: member.role, maxPermission: member.max_permission_mode };
  }
  if (chat && (chat.type === 'supergroup' || chat.type === 'group')) {
    const binding = await db.get(`SELECT * FROM group_bindings WHERE group_chat_id = ?`, [chat.id]);
    if (binding) {
      if (binding.owner_user_id === userId) return { role: 'owner', maxPermission: 'auto' };
      return { role: binding.default_member_role || 'viewer', maxPermission: 'ask' };
    }
  }
  return { role: 'viewer', maxPermission: 'plan' };
}

function parsePromptOverrides(rawText, conv, roleInfo) {
  let text = rawText;
  let harness = conv.harness;
  let model = conv.model;
  let thinking = conv.thinking_level;
  let permissions = conv.permission_mode;

  const tokens = text.split(/\s+/);
  while (tokens.length > 1 && tokens[0].startsWith('!')) {
    const flag = tokens.shift().slice(1).toLowerCase();
    if (THINKING_LEVELS.includes(flag)) thinking = flag;
    else if (PERMISSION_MODES.includes(flag)) permissions = flag;
    else if (HARNESS_MODELS[flag]) {
      harness = flag;
      model = HARNESS_MODELS[flag][0];
    }
  }
  text = tokens.join(' ');

  // Cap permission mode for non-owner Executors
  if (roleInfo.role === 'executor' && roleInfo.maxPermission !== 'auto' && permissions === 'auto') {
    permissions = roleInfo.maxPermission;
  }

  return { cleanText: text, harness, model, thinking, permissions };
}

function escapeHtml(str) {
  return String(str || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}
