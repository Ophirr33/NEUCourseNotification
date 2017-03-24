curl 'https://wl11gp.neu.edu/udcprod8/bwckschd.p_disp_listcrse?term_in=201760&subj_in=ENGW&crse_in=3302&crn_in=60356' 'https://wl11gp.neu.edu/udcprod8/bwckschd.p_disp_listcrse?term_in=201760&subj_in=ENGW&crse_in=3302&crn_in=60667' 'https://wl11gp.neu.edu/udcprod8/bwckschd.p_disp_listcrse?term_in=201760&subj_in=ENGW&crse_in=3315&crn_in=60415' 'https://wl11gp.neu.edu/udcprod8/bwckschd.p_disp_listcrse?term_in=201760&subj_in=ENGW&crse_in=3315&crn_in=60416' 'https://wl11gp.neu.edu/udcprod8/bwckschd.p_disp_listcrse?term_in=201760&subj_in=ENGW&crse_in=3315&crn_in=60417' 'https://wl11gp.neu.edu/udcprod8/bwckschd.p_disp_listcrse?term_in=201760&subj_in=ENGW&crse_in=3315&crn_in=60418' 'https://wl11gp.neu.edu/udcprod8/bwckschd.p_disp_listcrse?term_in=201760&subj_in=ENGW&crse_in=3315&crn_in=61035'

get_140() {
  SEATS_OPEN=$(sed "140q;d" | sed -n -E 's/<TD CLASS="dddefault">|<\/TD>//gp')
  NUM='^[0-9]+$'
  COURSE_NUM=$1
  COURSE_ID=$2

  if [[ $SEATS_OPEN == "0" ]]; then
    echo "Still no seats for $COURSE_ID"
    insert_into_table $COURSE_NUM $COURSE_ID $(($SEATS_OPEN + 0))
  elif [[ $SEATS_OPEN =~ $NUM ]]; then
    echo "Seats open! $SEATS_OPEN"
    insert_into_table $COURSE_NUM $COURSE_ID $(($SEATS_OPEN + 0))
    mail_good $COURSE_NUM $COURSE_ID $SEATS_OPEN
  else
    echo "NaN, html broken: `$SEATS_OPEN`"
    mail_bad $COURSE_NUM $COURSE_ID $SEATS_OPEN
  fi
}
