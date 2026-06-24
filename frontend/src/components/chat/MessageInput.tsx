import { useState, useEffect, useRef, type KeyboardEvent } from 'react';
import { useSessionStore } from '@store/sessionStore';
import { useChatStore } from '@store/chatStore';
import { useConfigStore } from '@store/configStore';
import * as ipc from '@bindings/ipc';
import { Textarea } from '@components/ui';
import { open } from '@tauri-apps/plugin-dialog';

export function MessageInput({ disabled, isStreaming, onCancel }: { disabled?: boolean; isStreaming?: boolean; onCancel?: () => void }) {
  const { activeSessionId } = useSessionStore();
  const { send } = useChatStore();
  const { config } = useConfigStore();
  const [text, setText] = useState('');
  const [provider, setProvider] = useState('');
  const [model, setModel] = useState('');
  const [ids, setIds] = useState<string[]>([]);
  const [models, setModels] = useState<Record<string,string>>({});
  const [attachedFile, setAttachedFile] = useState<{ name: string; content: string; language: string } | null>(null);
  const ref = useRef<HTMLTextAreaElement>(null);

  useEffect(() => { ipc.providerListAll().then(setIds).catch(()=>{}); }, []);
  useEffect(() => {
    if (!config) return;
    setProvider(config.agent.default_provider || 'openai');
    setModel(config.agent.default_model || 'gpt-4o-mini');
    const m: Record<string,string> = {};
    if (config.llm.openai.default_model) m['openai'] = config.llm.openai.default_model;
    if (config.llm.openrouter.default_model) m['openrouter'] = config.llm.openrouter.default_model;
    if (config.llm.anthropic.default_model) m['anthropic'] = config.llm.anthropic.default_model;
    if (config.llm.ollama.default_model) m['ollama'] = config.llm.ollama.default_model;
    for (const [id, c] of Object.entries(config.llm.custom || {})) { if (c.default_model) m[id] = c.default_model; }
    setModels(m);
  }, [config]);

  const handleProvider = (p: string) => { setProvider(p); if (models[p]) setModel(models[p]); };

  const handleAttachFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'All Files', extensions: ['*'] }],
      });
      if (!selected) return;
      const filePath = typeof selected === 'string' ? selected : selected.path;
      if (!filePath) return;
      const fileContent = await ipc.readFileForChat(filePath);
      if (fileContent.is_binary) {
        setText(prev => prev + `\n[Binary file: ${fileContent.filename}, ${fileContent.size} bytes]`);
        return;
      }
      setAttachedFile({
        name: fileContent.filename,
        content: fileContent.content,
        language: fileContent.language,
      });
    } catch (e) {
      console.error('File upload failed:', e);
    }
  };

  const submit = () => {
    const t = text.trim();
    if (!t && !attachedFile) return;
    if (!activeSessionId || disabled) return;

    let finalMessage = t;
    if (attachedFile) {
      finalMessage = `${t}\n\n--- File: ${attachedFile.name} ---\n\`\`\`${attachedFile.language}\n${attachedFile.content}\n\`\`\`\n--- End of ${attachedFile.name} ---`;
      setAttachedFile(null);
    }

    void send(activeSessionId, finalMessage, provider||undefined, model||undefined);
    setText('');
    if (ref.current) { ref.current.value=''; autoResize(); }
  };

  const autoResize = () => { const el = ref.current; if (!el) return; el.style.height='auto'; el.style.height=`${Math.min(el.scrollHeight,200)}px`; };
  const onKey = (e: KeyboardEvent<HTMLTextAreaElement>) => { if (e.key==='Enter'&&!e.shiftKey) { e.preventDefault(); submit(); } };
  useEffect(() => { ref.current?.focus(); }, [activeSessionId]);

  return (
    <div className="border-t border-space-700/50 bg-space-900/60 px-6 py-4 backdrop-blur">
      <div className="mx-auto max-w-3xl space-y-2">
        {/* Provider + Model selector */}
        <div className="flex items-center gap-2">
          <select value={provider} onChange={e=>handleProvider(e.target.value)} className="h-8 min-w-[120px] rounded-lg border border-space-600/50 bg-space-800/60 px-3 text-xs text-space-100 focus:outline-none focus:ring-2 focus:ring-aurora-500/40">
            {ids.length===0&&<option value="">Loading...</option>}
            {ids.map(id=><option key={id} value={id}>{id}</option>)}
          </select>
          <input type="text" value={model} onChange={e=>setModel(e.target.value)} placeholder="model" className="h-8 flex-1 rounded-lg border border-space-600/50 bg-space-800/60 px-3 text-xs text-space-100 focus:outline-none focus:ring-2 focus:ring-aurora-500/40"/>
        </div>

        {/* Attached file indicator */}
        {attachedFile && (
          <div className="flex items-center gap-2 rounded-lg border border-aurora-700/50 bg-aurora-900/10 px-3 py-1.5 text-xs">
            <span className="text-aurora-400">📎</span>
            <span className="flex-1 truncate text-space-200">{attachedFile.name}</span>
            <span className="text-space-500">{attachedFile.language}</span>
            <button onClick={() => setAttachedFile(null)} className="text-space-500 hover:text-danger">✕</button>
          </div>
        )}

        {/* Textarea + buttons */}
        <div className="flex items-end gap-2">
          {/* Attach file button */}
          <button
            onClick={handleAttachFile}
            disabled={disabled}
            className="flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-xl border border-space-600/50 bg-space-800/60 text-space-400 shadow-lg transition-all hover:scale-105 hover:text-aurora-400 disabled:opacity-40"
            title="Attach file for AI to read"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/>
            </svg>
          </button>

          <div className="flex-1"><Textarea ref={ref} value={text} onChange={e=>{setText(e.target.value);autoResize();}} onKeyDown={onKey} placeholder="Ask NEXUS anything... (Enter to send)" rows={1} disabled={disabled} className="min-h-[48px] max-h-[200px]"/></div>

          {isStreaming ? (
            <button onClick={onCancel} className="flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-xl bg-gradient-to-r from-danger to-pink-500 text-white shadow-lg"><span className="text-xs">■</span></button>
          ) : (
            <button onClick={submit} disabled={disabled||(!text.trim()&&!attachedFile)} className="flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-xl bg-gradient-to-r from-aurora-500 to-cyan-500 text-white shadow-lg disabled:opacity-40"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"/></svg></button>
          )}
        </div>
      </div>
    </div>
  );
}
