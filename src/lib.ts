import { Converter } from 'opencc-js';

export const BASE_ID = 'tongwen-s2tw';
export const opencc = Converter({ from: 'cn', to: 'tw' });

export interface Adapter {
  preprocess(input: string): string;
  suffix: string;
}

export interface ChatMessage {
  content: unknown;
  role: string;
}

export interface ChatRequest {
  messages: ChatMessage[];
  model?: string;
  stream?: boolean;
}

export function extractText(content: unknown): string {
  if (typeof content === 'string') return content;
  if (Array.isArray(content)) {
    return content
      .map((part) => {
        if (typeof part === 'string') return part;
        if (
          part && typeof part === 'object' &&
          typeof (part as { text?: unknown }).text === 'string'
        ) {
          return (part as { text: string }).text;
        }
        return '';
      })
      .join('');
  }
  return '';
}

export function pickInput(messages: ChatMessage[]): string {
  for (let i = messages.length - 1; i >= 0; i--) {
    if (messages[i].role === 'user') return extractText(messages[i].content);
  }
  return extractText(messages.at(-1)?.content);
}

function makeId(): string {
  return 'chatcmpl-' + crypto.randomUUID().replaceAll('-', '');
}

export function buildChatCompletion(opts: {
  input: string;
  output: string;
  model: string;
}) {
  return {
    id: makeId(),
    object: 'chat.completion',
    created: Math.floor(Date.now() / 1000),
    model: opts.model,
    choices: [{
      index: 0,
      message: { role: 'assistant', content: opts.output },
      finish_reason: 'stop',
    }],
    usage: {
      prompt_tokens: opts.input.length,
      completion_tokens: opts.output.length,
      total_tokens: opts.input.length + opts.output.length,
    },
  };
}

export function buildChatStream(opts: {
  output: string;
  model: string;
}): ReadableStream<Uint8Array> {
  const id = makeId();
  const created = Math.floor(Date.now() / 1000);
  const enc = new TextEncoder();
  const base = {
    id,
    object: 'chat.completion.chunk',
    created,
    model: opts.model,
  };

  return new ReadableStream<Uint8Array>({
    start(controller) {
      const send = (obj: unknown) =>
        controller.enqueue(enc.encode(`data: ${JSON.stringify(obj)}\n\n`));

      send({
        ...base,
        choices: [{
          index: 0,
          delta: { role: 'assistant' },
          finish_reason: null,
        }],
      });
      for (const ch of opts.output) {
        send({
          ...base,
          choices: [{ index: 0, delta: { content: ch }, finish_reason: null }],
        });
      }
      send({
        ...base,
        choices: [{ index: 0, delta: {}, finish_reason: 'stop' }],
      });
      controller.enqueue(enc.encode('data: [DONE]\n\n'));
      controller.close();
    },
  });
}
