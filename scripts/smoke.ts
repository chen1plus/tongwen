const base = Deno.env.get('TONGWEN_URL') ?? 'http://127.0.0.1:1180';

async function step(name: string, fn: () => Promise<void>) {
  try {
    await fn();
    console.log(`ok  ${name}`);
  } catch (e) {
    console.error(
      `FAIL ${name}: ${e instanceof Error ? e.message : String(e)}`,
    );
    Deno.exit(1);
  }
}

await step('GET /health', async () => {
  const r = await fetch(`${base}/health`);
  if (r.status !== 200) throw new Error(`status ${r.status}`);
  const t = await r.text();
  if (t !== 'ok') throw new Error(`body ${t}`);
});

await step('GET /v1/models', async () => {
  const r = await fetch(`${base}/v1/models`);
  const j = await r.json();
  if (!Array.isArray(j.data) || j.data.length === 0) {
    throw new Error('empty models');
  }
});

await step('POST /v1/chat/completions (non-stream)', async () => {
  const r = await fetch(`${base}/v1/chat/completions`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      messages: [{ role: 'user', content: '汉字转换：软件、电脑、网络' }],
    }),
  });
  const j = await r.json();
  const out = j.choices?.[0]?.message?.content;
  if (typeof out !== 'string') throw new Error('no content');
  if (!out.includes('漢字')) throw new Error(`expected 漢字, got: ${out}`);
  console.log(`     → ${out}`);
});

await step('POST /v1/chat/completions (stream)', async () => {
  const r = await fetch(`${base}/v1/chat/completions`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      stream: true,
      messages: [{ role: 'user', content: '简体变繁体' }],
    }),
  });
  if (!r.body) throw new Error('no body');
  const reader = r.body.getReader();
  const dec = new TextDecoder();
  let buf = '';
  let acc = '';
  let sawDone = false;
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    const lines = buf.split('\n');
    buf = lines.pop() ?? '';
    for (const line of lines) {
      if (!line.startsWith('data: ')) continue;
      const payload = line.slice(6).trim();
      if (payload === '[DONE]') {
        sawDone = true;
        continue;
      }
      const j = JSON.parse(payload);
      const delta = j.choices?.[0]?.delta?.content;
      if (typeof delta === 'string') acc += delta;
    }
  }
  if (!sawDone) throw new Error('missing [DONE]');
  if (!acc.includes('簡體')) throw new Error(`expected 簡體, got: ${acc}`);
  console.log(`     → ${acc}`);
});

console.log('\nall good ✓');
