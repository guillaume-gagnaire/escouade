<script lang="ts">
  import { t } from '../../lib/i18n';
  import { parseAgentMessage, parseTaskNotification, plainText } from '../../lib/events';
  import Markdown from './Markdown.svelte';

  // What Claude Code passed on to Claude by itself: `source` "task" (a background task), "agent"
  // (a subagent or another session), or another origin. `label`: the task a subagent was given.
  let { source, text, label = null }: { source: string; text: string; label?: string | null } = $props();

  // How a background task ended, as a whole sentence (the status the CLI reports is shown as is when it is another).
  function taskEnded(status: string): string {
    switch (status) {
      case 'completed':
        return t('conv.event.taskCompleted');
      case 'failed':
        return t('conv.event.taskFailed');
      case 'killed':
      case 'stopped':
        return t('conv.event.taskStopped');
      default:
        return t('conv.event.taskStatus', { status });
    }
  }

  const task = $derived(source === 'task' ? parseTaskNotification(text) : null);
  const message = $derived(source === 'agent' ? parseAgentMessage(text) : null);
  const line = $derived.by(() => {
    if (task?.status) return { what: taskEnded(task.status), detail: task.summary };
    // A notification that does not say a task ended (a scheduled trigger, a check-in…).
    if (task) return { what: t('conv.event.notification'), detail: task.summary ?? plainText(text) };
    return { what: t('conv.event.fromClaudeCode', { source }), detail: plainText(text) };
  });
  const title = $derived.by(() => {
    if (!message) return '';
    if (message.handback) return label ? t('conv.event.handbackNamed', { label }) : t('conv.event.handback');
    return label ? t('conv.event.messageNamed', { label }) : t('conv.event.message');
  });
</script>

{#if message}
  <div class="event agent">
    <div class="head"><span class="ico" aria-hidden="true">↩</span>{title}</div>
    <Markdown text={message.report} />
  </div>
{:else}
  <div class="event line" class:failed={task?.status === 'failed'}>
    <span class="ico" aria-hidden="true">◷</span>
    <span class="what">{line.what}</span>
    {#if line.detail}<span class="detail" title={line.detail}>{line.detail}</span>{/if}
  </div>
{/if}

<style>
  .event {
    margin-left: 34px;
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--dim);
    min-width: 0;
  }
  .line .what {
    flex: none;
    color: var(--muted);
  }
  .line.failed .what {
    color: var(--del);
  }
  .detail {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .agent {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 16px;
    border-radius: var(--r);
    border: 1px solid var(--line);
    border-left: 3px solid var(--accent);
    background: var(--panel);
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
</style>
