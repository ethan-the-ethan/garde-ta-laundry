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
    let progbar = "";

    let response = await fetch("/get/" + v[0]);
    if (response.status == 200) {
      let r = await response.json();
      let is_on = r.device.isOn;
      let last_changed = new Date(r.device.lastChanged);
      let last_updated = new Date(r.device.lastUpdated);

      let difference_formatted = generate_formatted_time_difference(last_updated, last_changed);
      last_updated_formatted = generate_formatted_time_difference(new Date(Date.now()), last_updated);

      let difference = Date.now() - last_updated.getTime();

      if (difference > 600000) {
        div_class = "available_timeout_div' onclick='alert(\"This machine has not recieved any updates for more than 10 minutes, so take this data with a pinch of salt!\")";
      } else {
        div_class = "not_in_use_div_two";
      }

      text = "<b>Available</b>"; // (for " + difference_formatted + ")";
      if (is_on) {
        if (difference > 600000) {
          div_class = "in_use_timeout_div' onclick=\'alert(\"This machine has not recieved any updates for more than 10 minutes, so take this data with a pinch of salt!\")";
        } else {
          div_class = "in_use_div_two";
        }
        text = "<b>In use</b> (for " + difference_formatted + ")"
      }

      // Generate history "progress bar" thing
      let column_string = "";
      let history = r.history;
      for (entry of history) {
        column_string += (" " + Math.round(entry[0]) + "%")
      }

      console.log(column_string);

      progbar += "<div class='progress_bar' style='display: grid; grid-template-columns: " + column_string + "; width: 100%; height: 5px; margin-top: 7px; border: 2px 50% white;'>"

      for (entry of history) {
        progbar += "<div style='background-color: " + (entry[1] ? "red" : "forestgreen") + "; height: 5px;'>&nbsp</div>";
      }

      progbar += "</div>"
    }

    inner_html += "<div class='machine_div'>" +
      "<div class='" + div_class + "'> <a style='float: left;'>" + v[1] + "</a><a style='float:right;'>" + text + "</a><br>" +
      progbar +
      "<div style='display:grid; grid-template-columns: 20% 60% 20%'>" +
      "<i style='font-size: 8pt; float: left; padding-top:5px; justify-self: left;'>2h ago</i>" +
      "<a style='font-size: 8pt; float: none; padding-top:5px; justify-self: center;'>Last updated " + last_updated_formatted + " ago</a>" +
      "<i style='font-size:8pt; float: right; padding-top:5px; justify-self: right;'>Now</i>" +
      "</div></div></div>";
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
