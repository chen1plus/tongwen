import { Converter } from 'opencc-js';

const HOST = Deno.env.get('TONGWEN_HOST') ?? '127.0.0.1';
const PORT = Number(Deno.env.get('TONGWEN_PORT') ?? 1180);

const BASE_MODEL_ID = 'tongwen-s2tw';
const NO_TAG_MODEL_ID = `${BASE_MODEL_ID}-no-tag`;
const MODEL_IDS = [BASE_MODEL_ID, NO_TAG_MODEL_ID];
const convert = Converter({ from: 'cn', to: 'tw' });

const TRANSCRIPT_TAG_RE = /<\/?TRANSCRIPT>/g;

function stripTranscriptTags(s: string): string {
  return s.replace(TRANSCRIPT_TAG_RE, '').trim();
}

interface ChatMessage {
  role: string;
  content: unknown;
}

interface ChatRequest {
  model?: string;
  messages: ChatMessage[];
  stream?: boolean;
}

function extractText(content: unknown): string {
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

function pickInput(messages: ChatMessage[]): string {
  for (let i = messages.length - 1; i >= 0; i--) {
    if (messages[i].role === 'user') return extractText(messages[i].content);
  }
  return extractText(messages.at(-1)?.content);
}

const jsonHeaders = { 'content-type': 'application/json; charset=utf-8' };

function json(data: unknown, status = 200) {
  return new Response(JSON.stringify(data), { status, headers: jsonHeaders });
}

function apiError(
  message: string,
  status = 400,
  type = 'invalid_request_error',
) {
  return json({ error: { message, type } }, status);
}

function makeId() {
  return 'chatcmpl-' + crypto.randomUUID().replaceAll('-', '');
}

async function handleChat(req: Request): Promise<Response> {
  let body: ChatRequest;
  try {
    body = await req.json();
  } catch {
    return apiError('Invalid JSON body');
  }
  if (!Array.isArray(body.messages) || body.messages.length === 0) {
    return apiError('`messages` must be a non-empty array');
  }

  const model = body.model || BASE_MODEL_ID;
  const raw = pickInput(body.messages);
  const input = model.endsWith('-no-tag') ? stripTranscriptTags(raw) : raw;
  const output = convert(input);
  const id = makeId();
  const created = Math.floor(Date.now() / 1000);

  if (body.stream) {
    const enc = new TextEncoder();
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        const send = (obj: unknown) =>
          controller.enqueue(enc.encode(`data: ${JSON.stringify(obj)}\n\n`));

        send({
          id,
          object: 'chat.completion.chunk',
          created,
          model,
          choices: [{
            index: 0,
            delta: { role: 'assistant' },
            finish_reason: null,
          }],
        });
        for (const ch of output) {
          send({
            id,
            object: 'chat.completion.chunk',
            created,
            model,
            choices: [{
              index: 0,
              delta: { content: ch },
              finish_reason: null,
            }],
          });
        }
        send({
          id,
          object: 'chat.completion.chunk',
          created,
          model,
          choices: [{ index: 0, delta: {}, finish_reason: 'stop' }],
        });
        controller.enqueue(enc.encode('data: [DONE]\n\n'));
        controller.close();
      },
    });
    return new Response(stream, {
      headers: {
        'content-type': 'text/event-stream; charset=utf-8',
        'cache-control': 'no-cache',
        'connection': 'keep-alive',
      },
    });
  }

  return json({
    id,
    object: 'chat.completion',
    created,
    model,
    choices: [{
      index: 0,
      message: { role: 'assistant', content: output },
      finish_reason: 'stop',
    }],
    usage: {
      prompt_tokens: input.length,
      completion_tokens: output.length,
      total_tokens: input.length + output.length,
    },
  });
}

function handleModels() {
  return json({
    object: 'list',
    data: MODEL_IDS.map((id) => ({
      id,
      object: 'model',
      created: 0,
      owned_by: 'tongwen',
    })),
  });
}

function handler(req: Request): Response | Promise<Response> {
  const url = new URL(req.url);
  if (req.method === 'OPTIONS') {
    return new Response(null, {
      status: 204,
      headers: {
        'access-control-allow-origin': '*',
        'access-control-allow-methods': 'POST, GET, OPTIONS',
        'access-control-allow-headers': 'authorization, content-type',
      },
    });
  }
  if (req.method === 'POST' && url.pathname === '/v1/chat/completions') {
    return handleChat(req);
  }
  if (req.method === 'GET' && url.pathname === '/v1/models') {
    return handleModels();
  }
  if (req.method === 'GET' && url.pathname === '/health') {
    return new Response('ok');
  }
  return apiError('Not Found', 404, 'not_found');
}

const server = Deno.serve({ port: PORT, hostname: HOST }, handler);

const shutdown = () => {
  server.shutdown().finally(() => Deno.exit(0));
};
Deno.addSignalListener('SIGTERM', shutdown);
Deno.addSignalListener('SIGINT', shutdown);
