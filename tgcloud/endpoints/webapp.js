/*
 * Personal and Educational Free License (PEF License / PEFL)
 * Version 1.0 - 2026-05-26
 * Copyright (c) 2026 Rubens Rainelli (rubens@rainelli.it)
 *
 * Personal and educational free use permitted.
 * Corporate, business or enterprise use prohibited without written authorization and fee.
 */

/**
 * Telegram Mini App Serverless Endpoint (`tgcloud/endpoints/webapp.js`)
 * Invoked via `Telegram.WebApp.Serverless.call('webapp', payload)` from the Mini App.
 *
 * `ctx.user.id` is cryptographically verified by Telegram Serverless V8 runtime,
 * guaranteeing strict multi-user isolation: a user can ONLY see projects they own
 * or projects where they have been explicitly invited (`viewer`, `proposer`, `executor`).
 */
export default async function handleWebAppCall(payload, ctx) {
  const db = ctx.db;
  const userId = ctx.user?.id;
  if (!userId) {
    return { ok: false, error: 'Unauthorized Telegram WebApp session' };
  }

  const action = payload?.action || 'dashboard';
  const now = Math.floor(Date.now() / 1000);

  if (action === 'dashboard') {
    const projects = await db.all(
      `SELECT DISTINCT p.*,
         CASE WHEN p.owner_user_id = ? THEN 'owner' ELSE COALESCE(pm.role, 'viewer') END AS my_role
       FROM projects p
       LEFT JOIN project_members pm ON pm.project_id = p.project_id AND pm.user_id = ?
       WHERE p.owner_user_id = ? OR pm.user_id = ?
       ORDER BY p.updated_at DESC`,
      [userId, userId, userId, userId]
    );

    const conversations = await db.all(
      `SELECT DISTINCT c.*
       FROM conversations c
       LEFT JOIN project_members pm ON pm.project_id = c.project_id AND pm.user_id = ?
       WHERE c.owner_user_id = ? OR pm.user_id = ?
       ORDER BY c.updated_at DESC LIMIT 30`,
      [userId, userId, userId]
    );

    const collaborators = await db.all(
      `SELECT pm.* FROM project_members pm
       JOIN projects p ON p.project_id = pm.project_id
       WHERE p.owner_user_id = ?`,
      [userId]
    );

    return {
      ok: true,
      userId,
      projects: projects || [],
      conversations: conversations || [],
      collaborators: collaborators || []
    };
  }

  if (action === 'conversation_detail') {
    const convId = payload.conversationId;
    const conv = await db.get(
      `SELECT c.*,
         CASE WHEN c.owner_user_id = ? THEN 'owner' ELSE COALESCE(pm.role, 'viewer') END AS my_role
       FROM conversations c
       LEFT JOIN project_members pm ON pm.project_id = c.project_id AND pm.user_id = ?
       WHERE c.conversation_id = ? AND (c.owner_user_id = ? OR pm.user_id = ?)`,
      [userId, userId, convId, userId, userId]
    );
    if (!conv) return { ok: false, error: 'Conversation not found or access denied' };

    return { ok: true, conversation: conv };
  }

  if (action === 'update_config') {
    const { conversationId, harness, model, thinkingLevel, permissionMode } = payload;
    const conv = await db.get(`SELECT * FROM conversations WHERE conversation_id = ?`, [conversationId]);
    if (!conv) return { ok: false, error: 'Conversation not found' };

    const member = await db.get(
      `SELECT role FROM project_members WHERE project_id = ? AND user_id = ?`,
      [conv.project_id, userId]
    );
    const role = conv.owner_user_id === userId ? 'owner' : (member?.role || 'viewer');
    if (role === 'viewer') return { ok: false, error: 'Read-only access (Viewer)' };

    await db.run(
      `UPDATE conversations SET harness = ?, model = ?, thinking_level = ?, permission_mode = ?, updated_at = ?
       WHERE conversation_id = ?`,
      [
        harness || conv.harness,
        model || conv.model,
        thinkingLevel || conv.thinking_level,
        permissionMode || conv.permission_mode,
        now,
        conversationId
      ]
    );
    return { ok: true };
  }

  if (action === 'set_collaborator_role') {
    const { projectId, targetUserId, targetUsername, role, maxPermissionMode } = payload;
    const project = await db.get(
      `SELECT * FROM projects WHERE project_id = ? AND owner_user_id = ?`,
      [projectId, userId]
    );
    if (!project) return { ok: false, error: 'Only the Project Owner can modify collaborator roles' };

    if (role === 'remove') {
      await db.run(`DELETE FROM project_members WHERE project_id = ? AND user_id = ?`, [projectId, targetUserId]);
    } else {
      await db.run(
        `INSERT INTO project_members (project_id, user_id, username, role, max_permission_mode, invited_by, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(project_id, user_id) DO UPDATE SET
           role = excluded.role,
           max_permission_mode = excluded.max_permission_mode`,
        [
          projectId,
          Number(targetUserId),
          targetUsername || '',
          role,
          maxPermissionMode || 'ask',
          userId,
          now
        ]
      );
    }
    return { ok: true };
  }

  return { ok: false, error: `Unknown webapp action '${action}'` };
}
