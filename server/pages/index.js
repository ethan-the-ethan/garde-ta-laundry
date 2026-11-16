function generate_formatted_time_difference(time_newest, time_oldest) {
  let difference = Math.round((time_newest.getTime() - time_oldest.getTime())) / 1000;
  let difference_mins = Math.floor(difference / 60);
  let difference_secs = difference % 60;
  let difference_hrs = Math.floor(difference_mins / 60);

  difference_mins -= (difference_hrs * 60)

  let difference_formatted = "";
  if (difference_hrs > 0) {
    difference_formatted = difference_hrs + "h " + difference_mins + "m " + Math.round(difference_secs) + "s";
  } else if (difference_mins > 0) {
    difference_formatted = difference_mins + "m " + Math.round(difference_secs) + "s";
  } else {
    difference_formatted = Math.round(difference_secs) + "s";
  }

  return difference_formatted;
}

async function fill_machine(machine_id) {
  let machine_div = document.getElementById(machine_id)

  let inner_html = "";

  let response = await fetch("/get/" + machine_id);

  if (response.status != 200) {
    inner_html += (machine_id + ": Failed to fetch data")
  } else {
    let r = await response.json();

    let display_name = r.display_name;
    let is_on = r.is_on;
    let last_changed = new Date(r.last_changed);
    let last_updated = new Date(r.last_updated);

    difference_formatted = generate_formatted_time_difference(last_updated, last_changed);
    last_updated_formatted = generate_formatted_time_difference(new Date(Date.now()), last_updated);

    let div_to_use = "not_in_use_div";
    let label_to_use = "Not in use";
    if (is_on) {
      div_to_use = "in_use_div";
      label_to_use = "In use"
    }

    let text = "<p><a class='" + div_to_use + "'> " + label_to_use + "</a> for " + difference_formatted + "</p>";

    inner_html += "<b>" + display_name + "</b> <span style='float:right'><i>Last updated " + last_updated_formatted + " ago</i></span>" + text;
  }

  machine_div.innerHTML = inner_html;
}

async function run() {
  let machines = [
    "THOMPSONS_FLOOR0_WASH",
    "THOMPSONS_FLOOR0_DRY",
    "THOMPSONS_FLOOR1_WASH",
    "THOMPSONS_FLOOR1_DRY",
    "THOMPSONS_FLOOR2_WASH",
    "THOMPSONS_FLOOR2_DRY"
  ]

  for (m of machines) {
    fill_machine(m)
  }
}

document.addEventListener("DOMContentLoaded", () => {
  run();
});

setInterval("run()", 30000);
