import { useState, useEffect } from 'react';
import { Dialog, Button, Input, Badge } from '@components/ui';
import * as ipc from '@bindings/ipc';
import type { AppConfig, CustomProviderConfig, CustomProviderDto } from '@bindings/types';
import clsx from 'clsx';

export function SettingsModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [tab, setTab] = useState<'providers'|'security'|'agent'>('providers');
  const [config, setConfig] = useState<AppConfig|null>(null);
  const [customs, setCustoms] = useState<CustomProviderDto[]>([]);
  const [allIds, setAllIds] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [showAdd, setShowAdd] = useState(false);
  const [presetOpen, setPresetOpen] = useState(false);

  useEffect(() => { if (open) load(); }, [open]);
  const load = async () => {
    setLoading(true);
    try {
      const [c, cs, ids] = await Promise.all([ipc.configGet(), ipc.customProviderList(), ipc.providerListAll()]);
      setConfig(c); setCustoms(cs); setAllIds(ids);
    } catch(e) { console.error(e); } finally { setLoading(false); }
  };

  return (
    <Dialog open={open} onClose={onClose} title="Settings" className="max-w-2xl">
      <div className="space-y-4">
        <div className="flex gap-1 border-b border-space-700/50">
          {(['providers','security','agent'] as const).map(t => (
            <button key={t} onClick={() => setTab(t)} className={clsx('px-4 py-2 text-sm font-medium border-b-2', tab===t?'border-aurora-500 text-aurora-300':'border-transparent text-space-400')}>{t==='providers'?'LLM Providers':t==='security'?'Security':'Agent'}</button>
          ))}
        </div>
        {loading && <p className="text-sm text-space-500">Loading...</p>}
        {!loading && tab==='providers' && config && (
          <div className="space-y-4">
            <div>
              <label className="mb-1 block text-xs font-medium text-space-400">Default Provider</label>
              <select value={config.agent.default_provider} onChange={async(e)=>{await ipc.configSet({agent:{default_provider:e.target.value}});load();}} className="h-10 w-full rounded-xl border border-space-600/50 bg-space-800/50 px-4 text-sm text-space-100">
                {allIds.map(id=><option key={id} value={id}>{id}</option>)}
              </select>
            </div>
            <div><label className="mb-1 block text-xs font-medium text-space-400">Default Model</label><Input value={config.agent.default_model} onChange={async(e)=>{await ipc.configSet({agent:{default_model:e.target.value}});load();}} placeholder="gpt-4o-mini"/></div>
            <div>
              <h3 className="mb-2 text-xs font-semibold uppercase text-space-500">Built-in Providers</h3>
              <div className="space-y-2">
                {(['openai','openrouter','anthropic','ollama'] as const).map(name => <ProviderRow key={name} name={name} pc={config.llm[name]} onChanged={load}/>)}
              </div>
            </div>
            <div>
              <div className="mb-2 flex items-center justify-between"><h3 className="text-xs font-semibold uppercase text-space-500">Custom Providers</h3><div className="flex gap-2"><Button size="sm" variant="ghost" onClick={()=>setPresetOpen(!presetOpen)}>Presets</Button><Button size="sm" variant="primary" onClick={()=>setShowAdd(!showAdd)}>+ Add</Button></div></div>
              {presetOpen && <div className="mb-3 grid grid-cols-3 gap-2">{['together','groq','mistral','deepseek','vllm','lmstudio','litellm'].map(p=><button key={p} onClick={async()=>{try{const c=await ipc.customProviderPreset(p);await ipc.customProviderAdd(c);setPresetOpen(false);load();}catch(e){console.error(e);}}} className="rounded-lg border border-space-700 bg-space-800/50 px-3 py-2 text-xs text-space-200 hover:bg-space-700/50">{p}</button>)}</div>}
              {showAdd && <AddForm onAdded={()=>{setShowAdd(false);load();}}/>}
              <div className="space-y-2">{customs.length===0&&!showAdd&&<p className="text-xs text-space-500">No custom providers.</p>}{customs.map(p=><div key={p.id} className="flex items-center justify-between rounded-lg border border-space-700 bg-space-800/50 px-3 py-2"><div><span className="text-sm font-medium text-space-100">{p.id}</span> {p.has_api_key?<Badge variant="success">key</Badge>:<Badge variant="warning">no key</Badge>}<p className="truncate text-2xs text-space-500">{p.base_url}</p></div><Button size="sm" variant="danger" onClick={async()=>{await ipc.customProviderRemove(p.id);load();}}>Remove</Button></div>)}</div>
            </div>
          </div>
        )}
        {!loading && tab==='security' && config && (
          <div className="space-y-4">
            <div><label className="mb-1 block text-xs font-medium text-space-400">Approval Timeout (s)</label><Input type="number" value={config.security.approval_timeout_secs} onChange={async(e)=>{await ipc.configSet({security:{approval_timeout_secs:Number(e.target.value)}});load();}}/></div>
            <div><label className="mb-1 block text-xs font-medium text-space-400">Shell Timeout (s)</label><Input type="number" value={config.security.shell_timeout_secs} onChange={async(e)=>{await ipc.configSet({security:{shell_timeout_secs:Number(e.target.value)}});load();}}/></div>
            <div><label className="mb-1 block text-xs font-medium text-space-400">Shell Max Output (KB)</label><Input type="number" value={config.security.shell_max_output_kb} onChange={async(e)=>{await ipc.configSet({security:{shell_max_output_kb:Number(e.target.value)}});load();}}/></div>
          </div>
        )}
        {!loading && tab==='agent' && config && (
          <div className="space-y-4">
            <div><label className="mb-1 block text-xs font-medium text-space-400">Max Iterations</label><Input type="number" value={config.agent.max_iterations} onChange={async(e)=>{await ipc.configSet({agent:{max_iterations:Number(e.target.value)}});load();}}/></div>
            <div><label className="mb-1 block text-xs font-medium text-space-400">Max Tool Calls</label><Input type="number" value={config.agent.max_tool_calls} onChange={async(e)=>{await ipc.configSet({agent:{max_tool_calls:Number(e.target.value)}});load();}}/></div>
            <div><label className="mb-1 block text-xs font-medium text-space-400">Default Provider</label><Input value={config.agent.default_provider} onChange={async(e)=>{await ipc.configSet({agent:{default_provider:e.target.value}});load();}}/></div>
            <div><label className="mb-1 block text-xs font-medium text-space-400">Default Model</label><Input value={config.agent.default_model} onChange={async(e)=>{await ipc.configSet({agent:{default_model:e.target.value}});load();}}/></div>
          </div>
        )}
      </div>
      <div className="mt-6 flex justify-end"><Button variant="ghost" onClick={onClose}>Close</Button></div>
    </Dialog>
  );
}

function ProviderRow({ name, pc, onChanged }: { name: string; pc: any; onChanged: () => void }) {
  const [showKey, setShowKey] = useState(false);
  const [key, setKey] = useState(pc.api_key ?? '');
  const [url, setUrl] = useState(pc.base_url ?? '');
  const [mdl, setMdl] = useState(pc.default_model ?? '');
  return (
    <div className="rounded-lg border border-space-700 bg-space-800/50 p-3">
      <div className="mb-2 flex items-center gap-2"><span className="text-sm font-medium text-space-100">{name}</span>{pc.api_key?<Badge variant="success">key set</Badge>:name!=='ollama'?<Badge variant="warning">no key</Badge>:null}</div>
      <div className="space-y-2">
        <div className="flex gap-1"><Input type={showKey?'text':'password'} value={key} onChange={e=>setKey(e.target.value)} placeholder={name+'_API_KEY'} className="text-xs"/><Button size="sm" variant="ghost" onClick={()=>setShowKey(!showKey)}>{showKey?'h':'e'}</Button></div>
        <div className="grid grid-cols-2 gap-2"><Input value={url} onChange={e=>setUrl(e.target.value)} placeholder="https://..." className="text-xs"/><Input value={mdl} onChange={e=>setMdl(e.target.value)} placeholder="model" className="text-xs"/></div>
        <Button size="sm" variant="secondary" onClick={async()=>{await ipc.configSet({llm:{[name]:{api_key:key||null,base_url:url||null,default_model:mdl||null}}});onChanged();}}>Save</Button>
      </div>
    </div>
  );
}

function AddForm({ onAdded }: { onAdded: () => void }) {
  const [id,setId]=useState(''); const [key,setKey]=useState(''); const [url,setUrl]=useState(''); const [mdl,setMdl]=useState('');
  const [saving,setSaving]=useState(false); const [err,setErr]=useState('');
  return (
    <div className="mb-3 rounded-lg border border-aurora-700/50 bg-aurora-900/10 p-3 space-y-2">
      <div className="grid grid-cols-2 gap-2"><Input value={id} onChange={e=>setId(e.target.value)} placeholder="ID" className="text-xs"/><Input value={url} onChange={e=>setUrl(e.target.value)} placeholder="https://api.../v1" className="text-xs"/></div>
      <Input type="password" value={key} onChange={e=>setKey(e.target.value)} placeholder="API Key" className="text-xs"/>
      <Input value={mdl} onChange={e=>setMdl(e.target.value)} placeholder="model-name" className="text-xs"/>
      {err&&<p className="text-xs text-danger">{err}</p>}
      <Button size="sm" variant="primary" disabled={saving||!id||!url} onClick={async()=>{setSaving(true);setErr('');try{const cfg:CustomProviderConfig={id,api_key:key||null,base_url:url,default_model:mdl||null,embedding_model:null,extra_headers:null,display_name:id,supports_tools:true,timeout_secs:120};await ipc.customProviderAdd(cfg);onAdded();}catch(e){setErr(e instanceof Error?e.message:String(e));}finally{setSaving(false);}}}>{saving?'Saving...':'Add'}</Button>
    </div>
  );
}
