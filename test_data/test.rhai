// Extract the sensor data from the mouse
let front_sensor = mouse.sensors["FRONT"];  // Sensor at the front
let left_diagonal_sensor = mouse.sensors["FRONT_LEFT"];  // Diagonal sensor on the left
let right_diagonal_sensor = mouse.sensors["FRONT_RIGHT"]; // Diagonal sensor on the right

// Initialize power for wheels
let left_power = 0.0;
let right_power = 0.0;

// Constants
let forward_threshold = 1500.0;  // Distance threshold to consider wall in front
let corner_threshold = 5.0;    // Distance threshold to consider a wall at corners
let turn_speed = 0.5;          // Speed at which the mouse should turn
let forward_speed = 0.8;       // Speed at which the mouse should move forward

// If there's a wall in front, decide to turn
if front_sensor.value < forward_threshold {
  // Check diagonal sensor distances to decide which way to turn
  if left_diagonal_sensor.value < corner_threshold {
      // Wall on the left side, turn right
      left_power = turn_speed;
      right_power = -turn_speed;
  } else if right_diagonal_sensor.value < corner_threshold {
      // Wall on the right side, turn left
      left_power = -turn_speed;
      right_power = turn_speed;
  } else {
      // If unsure, default to turning right
      left_power = turn_speed;
      right_power = -turn_speed;
  }
} else {
    // If there's no wall in front, move forward
    left_power = forward_speed;
    right_power = forward_speed;

    // Adjust course slightly based on diagonal sensor distances
    if left_diagonal_sensor.value < right_diagonal_sensor.value {
        // Slightly closer to left wall, nudge to the right
        right_power -= 0.05;
    } else if right_diagonal_sensor.value < left_diagonal_sensor.value {
        // Slightly closer to right wall, nudge to the left
        left_power -= 0.05;
    }
}

// Update the mouse's left and right power
mouse.left_power = left_power;
mouse.right_power = right_power;
