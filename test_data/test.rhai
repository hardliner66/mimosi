// Extract the sensor data from the mouse
let front_sensor = mouse.sensors["FRONT"];  // Sensor at the front
let left_diagonal_sensor = mouse.sensors["FRONT_LEFT"];  // Diagonal sensor on the left
let right_diagonal_sensor = mouse.sensors["FRONT_RIGHT"]; // Diagonal sensor on the right

let init = false;
if "last_tick" !in state {
  init = true;
  state.last_tick_left = mouse.left_encoder;
  state.last_tick_right = mouse.right_encoder;
}

// Initialize power for wheels
let left_power = 0.0;
let right_power = 0.0;

// Constants
let forward_threshold = 10000.0;  // Distance threshold to consider wall in front
let corner_threshold = 5.0;    // Distance threshold to consider a wall at corners
let turn_speed = 0.5;          // Speed at which the mouse should turn
let forward_speed = 0.8;       // Speed at which the mouse should move forward

// If there's a wall in front, decide to turn
print(front_sensor.value.to_string());
if front_sensor.value < forward_threshold {
    if !init {
      if mouse.left_encoder - state.left_last_tick < 10 {
        // Check diagonal sensor distances to decide which way to turn
        if left_diagonal_sensor.value < corner_threshold {
            print("Wall to the left, Turning right!");
            // Wall on the left side, turn right
            left_power = turn_speed;
            right_power = -turn_speed;
        } else if right_diagonal_sensor.value < corner_threshold {
            print("Wall to the right, Turning left!");
            // Wall on the right side, turn left
            left_power = -turn_speed;
            right_power = turn_speed;
        } else {
            print("unsure, Turning Right!");
            // If unsure, default to turning right
            left_power = turn_speed;
            right_power = -turn_speed;
        }
      } else {
        left_power = -1.0;
        right_power = -1.0;
      }
    }
} else {
    // If there's no wall in front, move forward
    left_power = forward_speed;
    right_power = forward_speed;

    // Adjust course slightly based on diagonal sensor distances
    if left_diagonal_sensor.value < right_diagonal_sensor.value {
        print("Left wall closer than right, steer right!");
        // Slightly closer to left wall, nudge to the right
        left_power += 0.2;
    } else if right_diagonal_sensor.value < left_diagonal_sensor.value {
        print("Right wall closer than left, steer left!");
        // Slightly closer to right wall, nudge to the left
        left_power -= 0.2;
    }
}

// Update the mouse's left and right power
mouse.left_power = left_power;
mouse.right_power = right_power;
