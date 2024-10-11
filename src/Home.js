import React from 'react';
import { Typography, Button, Container, Grid, Card, CardContent, CardActions } from '@mui/material';
import { useTheme } from '@mui/material/styles';

const HomePage = () => {
  const theme = useTheme();

  return (
    <Container maxWidth="lg" style={{ marginTop: '50px' }}>
      <Typography variant="h3" gutterBottom style={{ textAlign: 'center' }}>
        Köszöntünk a Kozmosz Közösségben!
      </Typography>
      <Typography variant="h6" gutterBottom style={{ textAlign: 'center' }}>
        Join us to explore the latest news, events, and discussions about the universe.
      </Typography>

      <Grid container spacing={3} style={{ marginTop: '30px' }}>
        {/* Row 1 */}
        <Grid item xs={12} sm={6}>
          <Card sx={{ height: '300px' }}>
            <CardContent>
              <Typography variant="h5">Community</Typography>
              <Typography variant="body2" color="text.secondary" sx={{ marginTop: '10px' }}>
                Connect with other space enthusiasts and share your love for the cosmos.
              </Typography>
            </CardContent>
            <CardActions>
              <Button size="small" color="primary">Join Now</Button>
            </CardActions>
          </Card>
        </Grid>

        <Grid item xs={12} sm={6}>
          <Card sx={{ height: '150px' }}> {/* Smaller card */}
            <CardContent>
              <Typography variant="h5">Upcoming Events</Typography>
              <Typography variant="body2" color="text.secondary" sx={{ marginTop: '10px' }}>
                Stay up-to-date with our event calendar.
              </Typography>
            </CardContent>
            <CardActions>
              <Button size="small" color="primary">View Events</Button>
            </CardActions>
          </Card>
        </Grid>

        {/* Row 2 */}
        <Grid item xs={12}>
          <Card sx={{ height: '400px' }}> {/* Full-width larger card */}
            <CardContent>
              <Typography variant="h5">Featured News</Typography>
              <Typography variant="body2" color="text.secondary" sx={{ marginTop: '10px' }}>
                Latest cosmic discoveries and theories.
              </Typography>
            </CardContent>
            <CardActions>
              <Button size="small" color="primary">Read More</Button>
            </CardActions>
          </Card>
        </Grid>

        {/* Row 3 */}
        <Grid item xs={12} sm={4}>
          <Card sx={{ height: '250px' }}> {/* Smaller card */}
            <CardContent>
              <Typography variant="h5">Discussions</Typography>
              <Typography variant="body2" color="text.secondary" sx={{ marginTop: '10px' }}>
                Join the conversation about space exploration.
              </Typography>
            </CardContent>
            <CardActions>
              <Button size="small" color="primary">Start Discussion</Button>
            </CardActions>
          </Card>
        </Grid>

        <Grid item xs={12} sm={8}>
          <Card sx={{ height: '250px' }}> {/* Larger card */}
            <CardContent>
              <Typography variant="h5">Latest Articles</Typography>
              <Typography variant="body2" color="text.secondary" sx={{ marginTop: '10px' }}>
                Explore the latest in space research and exploration.
              </Typography>
            </CardContent>
            <CardActions>
              <Button size="small" color="primary">Read Articles</Button>
            </CardActions>
          </Card>
        </Grid>
      </Grid>
    </Container>
  );
};

export default HomePage;
