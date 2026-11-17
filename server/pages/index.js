function generate_formatted_time_difference(time_newest, time_oldest) {
  let difference = Math.round((time_newest.getTime() - time_oldest.getTime())) / 1000;
  let difference_mins = Math.floor(difference / 60);
  let difference_hrs = Math.floor(difference_mins / 60);

  difference_mins -= (difference_hrs * 60)

  let difference_formatted = "";
  if (difference_hrs > 0) {
    difference_formatted = difference_hrs + "h " + difference_mins + "m";
  } else {
    difference_formatted = difference_mins + "m";
  }

  return difference_formatted;
}

async function fill_floor(floor_id, washer_machine_id, dryer_machine_id) {
  let inner_html = "";

  for (v of [[washer_machine_id, "Washer"], [dryer_machine_id, "Dryer"]]) {
    let div_class = "unknown_div";
    let text = "<b>Unknown</b>";
    let last_updated_formatted = "";

    let response = await fetch("/get/" + v[0]);
    if (response.status == 200) {
      let r = await response.json();
      let is_on = r.is_on;
      let last_changed = new Date(r.last_changed);
      let last_updated = new Date(r.last_updated);

      let difference_formatted = generate_formatted_time_difference(last_updated, last_changed);
      last_updated_formatted = generate_formatted_time_difference(new Date(Date.now()), last_updated);

      let difference = Date.now() - last_updated.getTime();

      if (difference > 600000) {
        div_class = "available_timeout_div";
      } else {
        div_class = "not_in_use_div_two";
      }

      text = "<b>Available</b> (for " + difference_formatted + ")";
      if (is_on) {
        if (difference > 600000) {
          div_class = "in_use_timeout_div";
        } else {
          div_class = "in_use_div_two";
        }
        text = "<b>In use</b> (for " + difference_formatted + ")"
      }
    }

    inner_html += "<div class='" + div_class + "'>" + v[1] + " <span class='last_updated_text'>Updated " + last_updated_formatted + " ago</span> <br>" + text + "</div>";
  }

  document.getElementById(floor_id).innerHTML = inner_html;
}

async function run() {
  await fill_floor("ground_floor", "THOMPSONS_FLOOR0_WASH", "THOMPSONS_FLOOR0_DRY");
  await fill_floor("first_floor", "THOMPSONS_FLOOR1_WASH", "THOMPSONS_FLOOR1_DRY");
  await fill_floor("top_floor", "THOMPSONS_FLOOR2_WASH", "THOMPSONS_FLOOR2_DRY");
}

document.addEventListener("DOMContentLoaded", () => {
  run();
});

setInterval("run()", 30000);
