import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useCapabilitiesStore } from '@/stores/capabilities';

describe('capabilities store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('marks cutthroat as available when health endpoint responds alive', async () => {
    fetch.mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ alive: true, service: 'cutthroat', version: '0.1.0' }),
    });
    const store = useCapabilitiesStore();

    await expect(store.refreshCutthroatAvailability()).resolves.toBe('available');
    expect(store.cutthroatAvailability).toBe('available');
  });

  it('marks cutthroat as unavailable when health endpoint fails', async () => {
    fetch.mockRejectedValue(new Error('network down'));
    const store = useCapabilitiesStore();

    await expect(store.refreshCutthroatAvailability()).resolves.toBe('unavailable');
    expect(store.cutthroatAvailability).toBe('unavailable');
  });
});
