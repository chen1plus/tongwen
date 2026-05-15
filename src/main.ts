import * as lib from './lib.ts';
import { voiceinkAdapter } from './voiceink.ts';

const HOST = Deno.env.get('TONGWEN_HOST') ?? '127.0.0.1';
const PORT = Number(Deno.env.get('TONGWEN_PORT') ?? 1180);

const adapters: lib.Adapter[] = [voiceinkAdapter];
const modelIds = [...adapters.map((a) => `${lib.BASE_ID}-${a.suffix}`), lib.BASE_ID];

function pickAdapter(model: string): lib.Adapter | null {
  return adapters.find((a) => model === `${lib.BASE_ID}-${a.suffix}`) ?? null;
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

async function handleChat(req: Request): Promise<Response> {
  let body: lib.ChatRequest;
  try {
    body = await req.json();
  } catch {
    return apiError('Invalid JSON body');
  }
  if (!Array.isArray(body.messages) || body.messages.length === 0) {
    return apiError('`messages` must be a non-empty array');
  }

  const model = body.model || lib.BASE_ID;
  const adapter = pickAdapter(model);
  const raw = lib.pickInput(body.messages);
  const input = adapter ? adapter.preprocess(raw) : raw;
  const output = lib.opencc(input);

  if (body.stream) {
    return new Response(lib.buildChatStream({ output, model }), {
      headers: {
        'content-type': 'text/event-stream; charset=utf-8',
        'cache-control': 'no-cache',
        'connection': 'keep-alive',
      },
    });
  }

  return json(lib.buildChatCompletion({ input, output, model }));
}

function handleModels() {
  return json({
    object: 'list',
    data: modelIds.map((id) => ({
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
