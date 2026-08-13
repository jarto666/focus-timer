import { createContext, type PropsWithChildren, useContext } from 'react';

import { runtimeConfig, type RuntimeConfig } from '@/config/runtime';

const RuntimeContext = createContext<RuntimeConfig | null>(null);

export function RuntimeProvider({ children }: PropsWithChildren) {
  return <RuntimeContext.Provider value={runtimeConfig}>{children}</RuntimeContext.Provider>;
}

export function useRuntimeConfig(): RuntimeConfig {
  const value = useContext(RuntimeContext);

  if (value === null) {
    throw new Error('useRuntimeConfig must be used inside RuntimeProvider');
  }

  return value;
}
