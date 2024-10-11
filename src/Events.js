import React, { useEffect, useState, useCallback } from 'react';
import { Typography, Container, Grid, Card, CardContent } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { Icon } from '@iconify/react';
import calendarIcon from '@iconify/icons-mdi/calendar-today-outline';
import timerIcon from '@iconify/icons-mdi/timer-outline';
import userIcon from '@iconify/icons-mdi/account-circle';
import participantsIcon from '@iconify/icons-mdi/account-multiple'; // New icon for participants

const EventsPage = () => {
  const [events, setEvents] = useState([]);
  const [timers, setTimers] = useState([]);
  const theme = useTheme();

  useEffect(() => {
    fetch('http://localhost:3001/events')
      .then((response) => response.json())
      .then((data) => {
        setEvents(data);
        updateTimers(data);
      })
      .catch((error) => console.error('Error fetching events:', error));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Memoize the updateTimers function to prevent unnecessary recalculations
  const updateTimers = useCallback((events) => {
    const newTimers = events.map((event) => calculateTMinus(event.date));
    setTimers(newTimers);
  }, []);

  const calculateTMinus = (eventDate) => {
    const now = new Date();
    const eventTime = new Date(eventDate);
    const diff = eventTime - now;

    if (diff <= 0) {
      return 'T - 0s';
    }

    const hours = Math.floor(diff / (1000 * 60 * 60));
    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
    const seconds = Math.floor((diff % (1000 * 60)) / 1000);

    const formattedMinutes = String(minutes).padStart(2, '0');
    const formattedSeconds = String(seconds).padStart(2, '0');

    return `T - ${hours}h ${formattedMinutes}m ${formattedSeconds}s`;
  };

  useEffect(() => {
    const interval = setInterval(() => {
      updateTimers(events);
    }, 1000);

    return () => clearInterval(interval);
  }, [events, updateTimers]);

  return (
    <Container maxWidth="md" style={{ marginTop: '50px' }}>
      <Typography variant="h3" gutterBottom>
        Események
      </Typography>

      <Grid container spacing={3}>
        {events.map((event, index) => (
          <Grid item xs={12} key={index}>
            <Card
              sx={{
                display: 'flex',
                position: 'relative',
                borderRadius: '15px',
                height: '200px',
                boxShadow: '0 8px 24px rgba(0,0,0,0.1)',
                backgroundColor: theme.palette.background.default,
              }}
            >
              {/* Event Image occupying full left side */}
              <div
                style={{
                  width: '20%',
                  height: '100%',
                  backgroundImage: `linear-gradient(to left, ${theme.palette.background.paper}, rgba(0, 0, 0, 0)), url(${event.image})`,
                  backgroundSize: 'cover',
                  backgroundPosition: 'center',
                  borderRadius: '15px 0 0 15px',
                  position: 'absolute',
                  left: 0,
                  zIndex: 1,
                }}
              />

              {/* Text Overlapping the Image */}
              <CardContent
                sx={{
                  position: 'relative',
                  display: 'flex',
                  flexDirection: 'column',
                  justifyContent: 'space-between',
                  top: 0,
                  left: '20%',
                  zIndex: 3,
                  color: 'white',
                  width: '60%',
                  paddingTop: '15px',
                  height: '100%',
                }}
              >
                {/* Event Date with Iconify Calendar Icon */}
                <Typography variant="body2" color="inherit" sx={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
                  <Icon icon={calendarIcon} style={{ fontSize: '1.5em' }} /> {event.date}
                </Typography>

                {/* Event Title */}
                <Typography variant="h6" sx={{ fontWeight: 'bold' }}>
                  {event.name}
                </Typography>

                {/* Event Description */}
                <Typography variant="body2" color="inherit" sx={{ marginBottom: '10px' }}>
                  {event.description || 'No description provided'}
                </Typography>

                {/* Event Organizer with User Icon */}
                <Typography
                  variant="body2"
                  color="inherit"
                  sx={{ marginTop: 'auto', paddingBottom: '25px', fontStyle: 'italic', display: 'flex', alignItems: 'center', gap: '5px' }}
                >
                  <Icon icon={userIcon} style={{ fontSize: '1.5em' }} /> {event.organizer}
                </Typography>
              </CardContent>

              {/* T-minus Timer and Participants in the Upper Right Corner */}
              <div
                style={{
                  position: 'absolute',
                  top: '10px',
                  right: '15px',
                  color: 'white',
                  fontWeight: 'bold',
                  backgroundColor: 'rgba(0, 0, 0, 0.5)',
                  padding: '5px 10px',
                  borderRadius: '10px',
                  zIndex: 4,
                }}
              >
                {/* Timer with Iconify Timer Icon */}
                <Typography variant="body2" sx={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
                  <Icon icon={timerIcon} style={{ fontSize: '1.5em' }} /> {timers[index]}
                </Typography>

                {/* Participants under the Timer with Icon */}
                <Typography variant="body2" sx={{ display: 'flex', alignItems: 'center', gap: '5px', marginTop: '5px' }}>
                  <Icon icon={participantsIcon} style={{ fontSize: '1.5em' }} /> {event.participants} résztvevő
                </Typography>
              </div>
            </Card>
          </Grid>
        ))}
      </Grid>
    </Container>
  );
};

export default EventsPage;
