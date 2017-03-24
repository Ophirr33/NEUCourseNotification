#!/bin/bash
SEATS_OPEN=$(sed "140q;d" | sed -n -E 's/<TD CLASS="dddefault">|<\/TD>//gp')
NUM='^[0-9]+$'

if [[ $SEATS_OPEN == "0" ]]; then
  echo "Still no seats"
elif [[ $SEATS_OPEN =~ $NUM ]]; then
  echo "Seats open! $SEATS_OPEN"
else
  echo "NaN, html broken: `$SEATS_OPEN`"
fi
