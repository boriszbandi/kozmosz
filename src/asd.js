const fs = require('fs');
const path = require('path');
const net = require('net');

// Function to get host IP address
function getHostIp() {
    let socket;
    try {
        socket = net.createConnection({ port: 80, host: '8.8.8.8' }); // Using Google's DNS as a more reliable IP
        return socket.localAddress || '127.0.0.1';
    } catch (err) {
        return '127.0.0.1';
    } finally {
        if (socket) {
            socket.end(); // Close the socket after retrieving the local address
        }
    }
}

// Function to parse duration strings (e.g., "2d 4h 10m 30s")
function parseDurationString(durationStr) {
    let days = 0, hours = 0, minutes = 0, seconds = 0;

    const parts = durationStr.split(' ');
    parts.forEach(part => {
        if (part.endsWith('d')) {
            days = parseInt(part.slice(0, -1));
        } else if (part.endsWith('h')) {
            hours = parseInt(part.slice(0, -1));
        } else if (part.endsWith('m')) {
            minutes = parseInt(part.slice(0, -1));
        } else if (part.endsWith('s')) {
            seconds = parseInt(part.slice(0, -1));
        }
    });

    return (days * 86400) + (hours * 3600) + (minutes * 60) + seconds;
}

// Convert duration string to epoch time
function convertToEpoch(durationStr, baseTime = Date.now()) {
    const totalSeconds = parseDurationString(durationStr);
    return Math.floor(baseTime / 1000) - totalSeconds;
}

// Class representing a basic TCP connection interface
class TcpLinuxInterface {
    constructor(ip, port) {
        this.ip = ip;
        this.port = port;
        this.socket = null;
    }

    async open() {
        return new Promise((resolve, reject) => {
            this.socket = net.createConnection({ host: this.ip, port: this.port }, () => {
                resolve();
            });
            this.socket.on('error', (err) => reject(err));
        });
    }

    async close() {
        if (this.socket) {
            this.socket.end();
        }
    }

    async send(command) {
        return new Promise((resolve, reject) => {
            this.socket.write(command, 'utf-8', (err) => {
                if (err) return reject(err);

                this.socket.on('data', (data) => {
                    resolve(data.toString());
                });
            });
        });
    }
}

// Function to read software information and generate Zabbix LLD JSON
function readSoftwareInfo(filePath, swName) {
    const discoveryData = { data: [] };
    const hostIp = getHostIp();

    try {
        const files = fs.readdirSync(filePath);
        
        files.forEach((filename) => {
            const _filePath = path.join(filePath, filename);
            const contents = fs.readFileSync(_filePath, 'utf8');
            const data = JSON.parse(contents);

            const name = data.name || 'Unknown';
            const address = data.address?.host || 'Unknown';
            const port = data.address?.port || 'Unknown';
            const alias = data.alias || 'Unknown';

            async function isServiceWorking(port) {
                try {
                    const tcpConn = new TcpLinuxInterface(address, port);
                    await tcpConn.open();

                    const commandOutput = await tcpConn.send('vars.get');
                    const infoGetOutput = await tcpConn.send('info.get');

                    const jsonContent = JSON.parse(commandOutput);
                    const infoGet = JSON.parse(infoGetOutput);

                    await tcpConn.close();

                    discoveryData.data.push({
                        "{#SOFTWARENAME}": name,
                        "{#PORT}": port,
                        "{#HOST}": hostIp,
                        "{#ALIAS}": alias,
                        "{#LONGNAME}": infoGet.longname,
                        "{#VERSION}": infoGet.version,
                        "{#DESCRIPTION}": infoGet.desc,
                        "{#UPTIME}": convertToEpoch(infoGet.uptime, 0),
                        "{#STARTEDAT}": convertToEpoch(infoGet.uptime),
                    });

                    return true;
                } catch (error) {
                    return false;
                }
            }

            isServiceWorking(port).then((serviceWorking) => {
                if (serviceWorking && name === swName) {
                    console.log(JSON.stringify(discoveryData, null, 2)); // Print result in Zabbix LLD format
                }
            });
        });
    } catch (err) {
        console.error(`Error reading software info: ${err}`);
    }
}

// Main execution block for Zabbix
if (process.argv.length === 3) {
    const swName = process.argv[2];
    const filePath = '/tmp/aitia/remotes'; // Directory containing the remote files for discovery
    readSoftwareInfo(filePath, swName);
} else {
    console.error(`Invalid arguments. Expected: software name.`);
}
