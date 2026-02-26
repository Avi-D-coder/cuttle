const Sails = require('sails').constructor;
const net = require('node:net');

function getAvailablePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const port = typeof address === 'object' && address ? address.port : null;
      server.close((err) => {
        if (err) {
          reject(err);
          return;
        }
        if (!Number.isInteger(port) || port <= 0) {
          reject(new Error('Failed to determine available test port.'));
          return;
        }
        resolve(port);
      });
    });
  });
}

export async function bootServer() {
  const configuredPort = Number(process.env.TEST_SAILS_PORT ?? 0);
  const port = Number.isInteger(configuredPort) && configuredPort > 0
    ? configuredPort
    : await getAvailablePort();
  return new Promise((resolve, reject) => {
    const sailsApp = new Sails();
    sailsApp.lift(
      {
        environment: 'development',
        port,
        log: {
          level: 'error',
        },
        hooks: {
          grunt: false,
        },
      },
      (err, server) => {
        if (err) {
          console.log('Sails error on bootwith error');
          console.log('\n\n', err, '\n\n');
          return reject(err);
        }

        return resolve(server);
      },
    );
  });
}

export function shutDownServer(server) {
  return new Promise((resolve, reject) => {

    if (!server) {
      return resolve();
    }

    server.lower((err) => {
      if (err) {
        console.log('\nFailed to lower sails\n');
        console.log(err, '\n\n');
        return reject(err);
      }

      return resolve();
    });
  });
}
