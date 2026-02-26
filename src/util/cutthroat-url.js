const DOCKER_SAILS_PORT = '1337';
const VITE_PROXY_PORT = '8080';
const LOOPBACK_HOSTS = new Set([ 'localhost', '127.0.0.1' ]);

function shouldUseViteProxy() {
  if (typeof window === 'undefined' || !window.location) {
    return false;
  }
  const { hostname, port } = window.location;
  return LOOPBACK_HOSTS.has(hostname) && port === DOCKER_SAILS_PORT;
}

export function resolveCutthroatHttpPath(path) {
  if (!shouldUseViteProxy()) {
    return path;
  }
  const { protocol, hostname } = window.location;
  return `${protocol}//${hostname}:${VITE_PROXY_PORT}${path}`;
}

export function resolveCutthroatWsUrl(path) {
  if (typeof window === 'undefined' || !window.location) {
    return path;
  }
  const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
  if (!shouldUseViteProxy()) {
    return `${protocol}://${window.location.host}${path}`;
  }
  return `${protocol}://${window.location.hostname}:${VITE_PROXY_PORT}${path}`;
}
