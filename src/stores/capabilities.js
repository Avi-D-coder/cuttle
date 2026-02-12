import { defineStore } from 'pinia';
import { ref } from 'vue';
import { resolveCutthroatHttpPath } from '@/util/cutthroat-url';

const CUTTHROAT_HEALTH_PATH = '/cutthroat/api/v1/health';
const CUTTHROAT_HEALTH_TIMEOUT_MS = 1200;
const AVAILABLE_CACHE_TTL_MS = 30000;
const UNAVAILABLE_RETRY_INITIAL_MS = 1000;
const UNAVAILABLE_RETRY_MAX_MS = 30000;

function withTimeout(fetchPromise, timeoutMs, controller) {
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
  return fetchPromise.finally(() => clearTimeout(timeoutId));
}

export const useCapabilitiesStore = defineStore('capabilities', () => {
  const cutthroatAvailability = ref('unknown');
  const cutthroatCheckedAt = ref(0);
  const cutthroatNextRetryAt = ref(0);
  const cutthroatFailureCount = ref(0);
  let cutthroatProbePromise = null;

  function markAvailable() {
    cutthroatAvailability.value = 'available';
    cutthroatCheckedAt.value = Date.now();
    cutthroatFailureCount.value = 0;
    cutthroatNextRetryAt.value = 0;
  }

  function markUnavailable() {
    cutthroatAvailability.value = 'unavailable';
    cutthroatCheckedAt.value = Date.now();
    cutthroatFailureCount.value += 1;
    const backoffMs = Math.min(
      UNAVAILABLE_RETRY_INITIAL_MS * (2 ** Math.max(0, cutthroatFailureCount.value - 1)),
      UNAVAILABLE_RETRY_MAX_MS,
    );
    cutthroatNextRetryAt.value = Date.now() + backoffMs;
  }

  function shouldSkipProbe(force) {
    if (force) {
      return false;
    }
    const now = Date.now();
    if (cutthroatAvailability.value === 'available') {
      return now - cutthroatCheckedAt.value < AVAILABLE_CACHE_TTL_MS;
    }
    if (cutthroatAvailability.value === 'unavailable') {
      return now < cutthroatNextRetryAt.value;
    }
    return false;
  }

  async function refreshCutthroatAvailability({ force = false } = {}) {
    if (shouldSkipProbe(force)) {
      return cutthroatAvailability.value;
    }
    if (cutthroatProbePromise) {
      return cutthroatProbePromise;
    }

    cutthroatProbePromise = (async () => {
      const controller = new AbortController();
      try {
        const healthUrl = resolveCutthroatHttpPath(CUTTHROAT_HEALTH_PATH);
        const response = await withTimeout(fetch(healthUrl, {
          method: 'GET',
          credentials: 'include',
          signal: controller.signal,
        }), CUTTHROAT_HEALTH_TIMEOUT_MS, controller);
        if (!response.ok) {
          markUnavailable();
          return cutthroatAvailability.value;
        }
        const data = await response.json().catch(() => null);
        if (data?.alive === true) {
          markAvailable();
        } else {
          markUnavailable();
        }
        return cutthroatAvailability.value;
      } catch (_) {
        markUnavailable();
        return cutthroatAvailability.value;
      } finally {
        cutthroatProbePromise = null;
      }
    })();

    return cutthroatProbePromise;
  }

  async function isCutthroatAvailable({ force = false } = {}) {
    const status = await refreshCutthroatAvailability({ force });
    return status === 'available';
  }

  return {
    cutthroatAvailability,
    refreshCutthroatAvailability,
    isCutthroatAvailable,
  };
});
