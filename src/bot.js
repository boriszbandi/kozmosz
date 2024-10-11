const express = require('express');
const cors = require('cors');  // Import cors
const { Client, GatewayIntentBits, Locale } = require('discord.js');
require('dotenv').config();

const client = new Client({
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildScheduledEvents,
  ],
});

const app = express();
const port = 3001;

let eventsData = [];

// Enable CORS for all routes
app.use(cors());  // Allow requests from any origin

// Function to fetch events
const fetchEvents = async () => {
    try {
      const guild = await client.guilds.fetch(process.env.DISCORD_GUILD_ID);  // Replace with your Discord server's ID
      const events = await guild.scheduledEvents.fetch();
  
      // Update the eventsData array
      eventsData = events.map(event => {
          // Use coverImageURL() to get the image URL if it exists
          const coverImageURL = event.coverImageURL({ size: 1024 }) || 'Default image URL';  // Provide a default or fallback image
  
          return {
            name: event.name,
            organizer: event.creator?.displayName || 'Unknown organizer', // Optional chaining in case creator is null
            image: coverImageURL,
            description: event.description || 'No description provided',
            date: new Date(event.scheduledStartTimestamp).toLocaleString(Locale.Hungarian),  // Convert to local time
            participants: event.userCount
          };
        });
  
      console.log(`Events fetched at ${new Date().toLocaleTimeString()}:`);
      eventsData.forEach(event => console.log(`- ${event.name}, Date: ${event.date}, Image: ${event.image}, Participants: ${event.participants}`));
    } catch (error) {
      console.error('Error fetching events:', error);
    }
  };

// Call fetchEvents once on startup
client.once('ready', () => {
  console.log('Bot is ready!');
  fetchEvents();  // Fetch events immediately on startup

  // Set up interval to fetch events every 1 minute
  setInterval(fetchEvents, 60000);  // 60000 milliseconds = 1 minute
});

// API endpoint to serve events
app.get('/events', (req, res) => {
  res.json(eventsData);
});

app.listen(port, () => {
  console.log(`Server running on port ${port}`);
});

// Log in the bot
client.login(process.env.DISCORD_BOT_TOKEN);  // Replace with your bot token from Discord Developer Portal
