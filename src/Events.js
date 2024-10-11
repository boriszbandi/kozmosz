import React, { useEffect, useState } from 'react';
import { Typography, Container, Grid, Card, CardContent, CardHeader, CardMedia, IconButton } from '@mui/material';

const EventsPage = () => {
  const [events, setEvents] = useState([]);

  useEffect(() => {
    fetch('http://localhost:3001/events')
      .then(response => response.json())
      .then(data => setEvents(data))
      .catch(error => console.error('Error fetching events:', error));
  }, []);

  const getRandomImage = () => {
    // Replace this with actual event-specific images or an image API
    const images = [
      'https://via.placeholder.com/150/0000FF', // Replace with your own images or a relevant API
      'https://via.placeholder.com/150/FF0000',
      'https://via.placeholder.com/150/00FF00',
    ];
    return images[Math.floor(Math.random() * images.length)];
  };

  return (
    <Container maxWidth="md" style={{ marginTop: '50px' }}>
      <Typography variant="h3" gutterBottom>
        Események
      </Typography>

      <Grid container spacing={3}>
        {events.map((event, index) => (
          <Grid item xs={12} md={4} key={index}>
            <Card sx={{ borderRadius: '15px', boxShadow: '0 8px 24px rgba(0,0,0,0.1)' }}>
              <CardMedia
                component="img"
                height="150"
                image={getRandomImage()}  // Replace with event-specific images
                alt="Event Image"
                sx={{ borderRadius: '15px 15px 0 0' }}
              />
              <CardHeader
                title={<Typography variant="h6" style={{ fontWeight: 'bold' }}>{event.name}</Typography>}
                subheader={<Typography variant="body2" color="textSecondary">🕒 {event.date}</Typography>}
                sx={{ textAlign: 'center' }}
              />
              <CardContent sx={{ textAlign: 'center' }}>
                <Typography variant="body2" color="textSecondary" sx={{ marginBottom: '10px' }}>
                  {event.description}
                </Typography>
                
                {/* Icons for additional information */}
                <Grid container justifyContent="center" alignItems="center" spacing={1}>
                  {/* Ethereum price icon or other event-specific data */}
                  <Grid item>
                    <IconButton size="small" disabled>                      
                    </IconButton>
                  </Grid>
                  <Grid item>
                    <Typography variant="body2" color="textSecondary">0.041 ETH</Typography>
                  </Grid>

                  {/* Time left icon */}
                  <Grid item>
                    <IconButton size="small" disabled>                     
                    </IconButton>
                  </Grid>
                  <Grid item>
                    <Typography variant="body2" color="textSecondary">3 days left</Typography>
                  </Grid>

                  {/* Location icon */}
                  <Grid item>
                    <IconButton size="small" disabled>                      
                    </IconButton>
                  </Grid>
                  <Grid item>
                    <Typography variant="body2" color="textSecondary">📍 {event.location || 'Not specified'}</Typography>
                  </Grid>
                </Grid>
              </CardContent>
            </Card>
          </Grid>
        ))}
      </Grid>
    </Container>
  );
};

export default EventsPage;
