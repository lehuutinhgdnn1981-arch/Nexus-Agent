import { useEffect } from 'react';
import { onSchedulerFired } from '@bindings/ipc';
import { useSchedulerStore } from '@store/schedulerStore';

export function useSchedulerEvents() {
  const load = useSchedulerStore((s) => s.load);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onSchedulerFired((_payload) => {
      // Refresh scheduler list when a job fires
      load();
    })
      .then((un) => {
        unlisten = un;
      })
      .catch((e) => {
        console.error('Failed to subscribe to scheduler events:', e);
      });

    return () => {
      if (unlisten) unlisten();
    };
  }, [load]);
}
